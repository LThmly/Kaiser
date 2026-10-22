//! Account selection, sign-in tasks, and provider activation.
use super::{AccountTaskResult, App, Modal, Provider, mpsc};

impl App {
    pub fn initialize_auth(&mut self, store: crate::auth::Store) {
        let current = store.current();
        self.auth = Some(store);
        if let Err(error) = self.refresh_accounts() {
            self.status = error;
        }
        match current {
            Ok(Some(connection)) => self.connect_account(connection),
            Err(error) => self.status = error,
            _ => {}
        }
    }
    pub(super) fn refresh_accounts(&mut self) -> Result<(), String> {
        self.accounts = self
            .auth
            .as_ref()
            .ok_or("ChatGPT sign-in is unavailable in demo mode")?
            .summaries()?;
        Ok(())
    }
    pub(super) fn connect_account(&mut self, connection: crate::auth::Connection) {
        self.cancel_generation();
        self.chatgpt_models.clear();
        self.provider = if connection.plan {
            self.auth.as_ref().map(|store| {
                Provider::chatgpt(
                    store.clone(),
                    connection.client_id.clone(),
                    connection.model.clone(),
                )
                .with_usage(self.ai.clone())
            })
        } else {
            None
        };
        self.status = "Signed in".into();
        let welcome = connection.welcome;
        self.chatgpt = Some(connection);
        if welcome {
            self.modal = Some(Modal::ChatgptWelcome);
        }
        if let Err(error) = self.refresh_accounts() {
            self.status = error;
        }
    }
    pub(super) fn account_menu(&mut self) {
        if self.login.is_some() || self.account_task.is_some() {
            self.status = "A ChatGPT account operation is already running".into();
            return;
        }
        match self.refresh_accounts() {
            Ok(()) => {
                self.modal_index = 0;
                self.modal = Some(Modal::Accounts);
            }
            Err(error) => self.status = error,
        }
    }
    pub(super) fn start_login(&mut self, account: Option<String>) {
        if self.login.is_some() || self.account_task.is_some() {
            return;
        }
        let Some(store) = &self.auth else {
            self.status = "ChatGPT sign-in is unavailable in demo mode".into();
            return;
        };
        self.login = Some(store.login(account));
        self.modal = Some(Modal::SignIn(String::new()));
        self.status = "Connecting to ChatGPT".into();
    }
    pub(super) fn load_models(&mut self) {
        if self.login.is_some() || self.account_task.is_some() {
            self.status = "A ChatGPT account operation is already running".into();
            return;
        }
        let (Some(store), Some(connection)) = (&self.auth, &self.chatgpt) else {
            self.account_menu();
            return;
        };
        let store = store.clone();
        let id = connection.client_id.clone();
        let (sender, receiver) = mpsc::channel();
        self.account_task = Some(receiver);
        self.modal = Some(Modal::ChatgptModels);
        self.chatgpt_models.clear();
        self.modal_index = 0;
        std::thread::spawn(move || {
            let _ = sender.send(store.models(&id).map(AccountTaskResult::Models));
        });
    }
    pub(super) fn sign_out(&mut self) {
        // Drop the login receiver and stop accepting any in-flight tutor answer.
        self.login = None;
        if self.account_task.is_some() {
            self.status = "Wait for the current ChatGPT account operation to finish".into();
            return;
        }
        let Some(store) = &self.auth else {
            return;
        };
        let store = store.clone();
        self.cancel_generation();
        self.provider = self.fallback_provider.clone();
        self.chatgpt = None;
        self.chatgpt_models.clear();
        self.modal = None;
        self.status = "Signing out of ChatGPT".into();
        let (sender, receiver) = mpsc::channel();
        self.account_task = Some(receiver);
        std::thread::spawn(move || {
            let _ = sender.send(store.logout().map(AccountTaskResult::SignedOut));
        });
    }
    pub fn poll_account(&mut self) {
        if self.login.is_some() && !matches!(self.modal, Some(Modal::SignIn(_))) {
            self.login = None;
        }
        if let Some(result) = self.login.as_ref().map(|job| job.receiver.try_recv()) {
            match result {
                Ok(crate::auth::Event::Waiting(url)) => {
                    self.modal = Some(Modal::SignIn(url));
                }
                Ok(crate::auth::Event::Finished(result)) => {
                    self.login = None;
                    if matches!(self.modal, Some(Modal::SignIn(_))) {
                        self.modal = None;
                    }
                    let result = result.and_then(|authorized| {
                        self.auth
                            .as_ref()
                            .ok_or("ChatGPT account storage is unavailable".to_owned())?
                            .commit(authorized)
                    });
                    match result {
                        Ok(connection) => self.connect_account(connection),
                        Err(error) => self.status = error,
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.login = None;
                    self.modal = None;
                    self.status = "ChatGPT sign-in stopped".into();
                }
                _ => {}
            }
        }
        if let Some(result) = self.account_task.as_ref().map(|job| job.try_recv()) {
            match result {
                Ok(result) => {
                    self.account_task = None;
                    match result {
                        Ok(AccountTaskResult::Models(models)) => {
                            self.chatgpt_models = models;
                            if matches!(self.modal, Some(Modal::ChatgptModels)) {
                                self.modal_index = 0;
                            }
                        }
                        Ok(AccountTaskResult::SignedOut(message)) => {
                            self.status = message;
                            let _ = self.refresh_accounts();
                        }
                        Err(error) => {
                            self.status = error;
                            if matches!(self.modal, Some(Modal::ChatgptModels)) {
                                self.modal = None;
                            }
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.account_task = None;
                    self.status = "ChatGPT account operation stopped".into();
                }
                _ => {}
            }
        }
    }
}
