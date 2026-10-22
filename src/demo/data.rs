//! Static, fictional question/answer pairs for the disposable showcase.
use super::Fact;

pub(super) const DEEP_LEARNING: &[Fact] = &[
    (
        "What does a hidden layer learn?",
        "A hidden layer transforms its inputs into a representation useful to later layers. Training adjusts its weights so those representations help minimize the task's loss.",
    ),
    (
        "Why do neural networks need nonlinear activations?",
        "Without nonlinear activations, a stack of linear layers is still a single linear transformation. Nonlinearity allows a network to represent more complex relationships.",
    ),
    (
        "What is a forward pass?",
        "A forward pass applies each layer to an input to produce a prediction. The loss then measures how that prediction differs from the desired output.",
    ),
    (
        "What does backpropagation compute?",
        "Backpropagation uses the chain rule to compute gradients of the loss with respect to model parameters. An optimizer uses those gradients to update the parameters.",
    ),
    (
        "Why can a model fit its training data but fail on new examples?",
        "The model may be overfitting: learning patterns specific to the training set that do not generalize. Compare training and validation loss, then consider more representative data, regularization, or a simpler model.",
    ),
    (
        "What is the purpose of a validation set?",
        "A validation set estimates performance on held-out data during development. Use it to compare model choices and tune hyperparameters without repeatedly inspecting the final test set.",
    ),
    (
        "How does gradient descent update a parameter?",
        "Subtract the learning rate multiplied by the parameter's loss gradient. The gradient gives a local direction of increasing loss, so the update moves in the opposite direction.",
    ),
    (
        "What can happen when the learning rate is too large?",
        "Updates can overshoot useful regions, cause oscillation, or make training diverge. Inspect loss curves and try a smaller rate or an appropriate learning-rate schedule.",
    ),
    (
        "What does a mini-batch control during training?",
        "A mini-batch determines the examples used to estimate the gradient for one update. Its size affects memory use, throughput, and the noise in gradient estimates.",
    ),
    (
        "How are an epoch and an optimization step different?",
        "An epoch is one pass through the training dataset. A step is one parameter update, usually using one mini-batch; an epoch typically contains many steps.",
    ),
    (
        "What does dropout do?",
        "During training, dropout randomly masks some activations. This discourages reliance on particular activations and can reduce overfitting; ordinary inference uses the full network with the appropriate scaling.",
    ),
    (
        "What is early stopping?",
        "Early stopping ends training when a monitored validation metric stops improving according to a chosen patience rule. Restore the best checkpoint rather than necessarily using the final update.",
    ),
    (
        "How does L2 regularization affect the objective?",
        "L2 regularization adds a penalty proportional to the squared magnitude of parameters. It discourages large weights and changes the tradeoff between fitting the data and controlling model complexity.",
    ),
    (
        "Why is cross-entropy useful for classification?",
        "Cross-entropy penalizes assigning low probability to the correct class. It connects probabilistic predictions to an objective that can be optimized during training.",
    ),
    (
        "What does softmax produce?",
        "Softmax converts a vector of real-valued scores into positive values summing to one. These values are commonly interpreted as predicted probabilities over mutually exclusive classes.",
    ),
    (
        "Why should the test set remain separate from model selection?",
        "Repeatedly using test results to choose a model leaks information into development decisions. Reserve a final test set for estimating performance after model selection.",
    ),
    (
        "What is an embedding?",
        "An embedding represents an item as a vector. Learned embeddings can place items with useful similarities near each other for a particular training objective.",
    ),
    (
        "What is the role of a convolutional filter?",
        "A convolutional filter applies shared weights across local regions of an input. This lets the same feature detector respond at different positions.",
    ),
    (
        "What is transfer learning?",
        "Transfer learning adapts knowledge from a previously trained model to a new task or dataset. A common approach starts from pretrained weights and trains a task-specific head or fine-tunes the model.",
    ),
    (
        "What does freezing a layer mean?",
        "Freezing a layer keeps its parameters fixed during optimization. Other layers can continue learning while the frozen layer supplies a stable representation.",
    ),
    (
        "Why can feature normalization help optimization?",
        "Bringing input features to comparable scales can improve the conditioning of the optimization problem and make it easier to choose useful update sizes.",
    ),
    (
        "What is data leakage?",
        "Data leakage occurs when training or model selection uses information that would not be available at prediction time. It can make evaluation look stronger than real deployment performance.",
    ),
    (
        "Why should a baseline model come before a complex network?",
        "A simple baseline gives a reference for evaluating whether additional complexity helps. It can also reveal data problems and set expectations for useful performance.",
    ),
    (
        "How would you diagnose a widening training-validation loss gap?",
        "Check for overfitting, distribution differences, and inconsistent preprocessing. Compare representative examples and controlled experiments before changing several hyperparameters at once.",
    ),
];
pub(super) const SYSTEMS: &[Fact] = &[
    (
        "What is replication?",
        "Replication maintains copies of data on multiple nodes to support availability, durability, or read capacity.",
    ),
    (
        "Why can asynchronous replicas serve stale reads?",
        "A leader may acknowledge a write before every replica receives it. Reads from a lagging replica can return an older value.",
    ),
    (
        "What does read-your-writes consistency guarantee?",
        "A client can observe its own completed writes in subsequent reads.",
    ),
    (
        "What does eventual consistency promise?",
        "If updates stop and communication succeeds, replicas eventually converge. It does not require every read to return the latest write immediately.",
    ),
    (
        "What is a network partition?",
        "A network partition prevents some nodes from communicating even though those nodes may still be running.",
    ),
    (
        "What makes an operation idempotent?",
        "Repeating an idempotent operation has the same intended effect as applying it once.",
    ),
    (
        "Why use an idempotency key for retries?",
        "The key lets a service recognize repeated attempts for the same logical request and avoid duplicating its effects.",
    ),
    (
        "What is a quorum?",
        "A quorum is a required subset of participants whose responses are sufficient for an operation under the system's protocol.",
    ),
    (
        "Why add jitter to retry delays?",
        "Jitter spreads retry attempts over time so many clients are less likely to retry in synchronized bursts.",
    ),
    (
        "What is a write-ahead log?",
        "A write-ahead log records changes before applying them to durable data structures, supporting recovery after a crash.",
    ),
    (
        "What is backpressure?",
        "Backpressure signals that a consumer cannot keep up, allowing producers to slow down or apply a bounded overload policy.",
    ),
    (
        "Why distinguish latency from throughput?",
        "Latency measures how long an operation takes; throughput measures how many operations complete per unit of time. Improving one does not automatically improve the other.",
    ),
];
pub(super) const ALGEBRA: &[Fact] = &[
    (
        "What does a dot product measure?",
        "The dot product combines corresponding vector components into a scalar. Geometrically, it relates vector lengths to the cosine of the angle between them.",
    ),
    (
        "When are two vectors orthogonal?",
        "Two vectors are orthogonal when their dot product is zero.",
    ),
    (
        "What does a matrix represent as a linear map?",
        "A matrix represents a transformation that preserves vector addition and scalar multiplication.",
    ),
    (
        "What is the rank of a matrix?",
        "Rank is the dimension of the matrix's column space, equivalently its row space.",
    ),
    (
        "What is the null space?",
        "The null space contains all input vectors mapped to the zero vector.",
    ),
    (
        "What is an eigenvector?",
        "An eigenvector is a nonzero vector whose direction is unchanged by a square linear transformation; its eigenvalue gives the scale factor.",
    ),
    (
        "What does a determinant of zero imply?",
        "A square matrix with determinant zero is singular and has no inverse.",
    ),
    (
        "What does an orthogonal projection find?",
        "An orthogonal projection finds the closest vector in a subspace under Euclidean distance.",
    ),
    (
        "What is a basis?",
        "A basis is a linearly independent set of vectors that spans a vector space.",
    ),
    (
        "What does singular value decomposition factorize?",
        "SVD writes a matrix as U times Sigma times V-transpose, with orthonormal singular vectors and nonnegative singular values.",
    ),
    (
        "What is the Euclidean norm?",
        "The Euclidean norm is the square root of the sum of squared vector components.",
    ),
    (
        "What does a transpose do?",
        "A transpose exchanges a matrix's rows and columns.",
    ),
];
pub(super) const RUST: &[Fact] = &[
    (
        "What does ownership associate with a value?",
        "Ownership associates a value with an owner responsible for its lifetime and eventual cleanup.",
    ),
    (
        "How does borrowing differ from moving?",
        "Borrowing grants access through a reference while ownership stays in place. A move transfers ownership of a value.",
    ),
    (
        "What is the core shared-versus-mutable borrowing rule?",
        "For the same data, Rust permits multiple shared references or exclusive mutable access, subject to the borrow checker's lifetime rules.",
    ),
    (
        "What does Option represent?",
        "Option represents either a present value with Some or an absent value with None.",
    ),
    (
        "What does Result represent?",
        "Result represents success with Ok or a recoverable failure with Err.",
    ),
    (
        "What does the question-mark operator do?",
        "The question-mark operator unwraps a successful compatible result or returns early with its failure, applying the supported conversion.",
    ),
    (
        "What is a trait?",
        "A trait defines behavior that types can implement, allowing shared interfaces and generic bounds.",
    ),
    (
        "Why use an enum for application states?",
        "An enum represents a finite set of alternatives and enables exhaustive matching over those states.",
    ),
    (
        "What does a lifetime annotation describe?",
        "A lifetime annotation describes relationships between reference lifetimes; it does not extend how long a value lives.",
    ),
    (
        "What is the purpose of Drop?",
        "Drop runs cleanup when a value is destroyed, supporting deterministic resource management.",
    ),
    (
        "What is pattern matching useful for?",
        "Pattern matching destructures values and selects behavior by shape, with exhaustiveness checks for match expressions.",
    ),
    (
        "When is a bounded channel useful?",
        "A bounded channel limits queued messages and can apply backpressure when producers outpace consumers.",
    ),
];
pub(super) const ALGORITHMS: &[Fact] = &[
    (
        "What is a loop invariant?",
        "A loop invariant is a property maintained before and after each iteration, helping establish correctness.",
    ),
    (
        "What prerequisite does binary search have?",
        "Binary search requires an ordered search space and a comparison that lets each step discard a portion safely.",
    ),
    (
        "What is the time complexity of binary search?",
        "Binary search uses logarithmically many comparisons in the size of its ordered search space.",
    ),
    (
        "What does breadth-first search explore first?",
        "Breadth-first search explores nodes by distance in edges from the start in an unweighted graph.",
    ),
    (
        "What data structure naturally supports depth-first search?",
        "A stack, either explicit or supplied by recursive calls, supports depth-first traversal.",
    ),
    (
        "Why can a hash-table lookup have poor worst-case performance?",
        "Collisions can place many keys in the same search structure. The exact worst case depends on the table's collision-handling implementation.",
    ),
    (
        "What is memoization?",
        "Memoization caches computed results so repeated subproblems can reuse them.",
    ),
    (
        "What distinguishes dynamic programming from plain recursion?",
        "Dynamic programming reuses solutions to overlapping subproblems instead of repeatedly solving them independently.",
    ),
    (
        "What makes a sorting algorithm stable?",
        "A stable sort preserves the original relative order of elements whose sort keys compare equal.",
    ),
    (
        "When does Dijkstra's shortest-path algorithm apply?",
        "Dijkstra's algorithm computes shortest paths in graphs with nonnegative edge weights.",
    ),
    (
        "What is amortized analysis?",
        "Amortized analysis bounds the total cost of a sequence of operations, allowing some individual operations to be expensive.",
    ),
    (
        "What does a priority queue return?",
        "A priority queue returns an item with the highest or lowest priority according to its ordering.",
    ),
];
pub(super) const STATISTICS: &[Fact] = &[
    (
        "How do a population and a sample differ?",
        "A population is the full group of interest; a sample is the subset observed for an analysis.",
    ),
    (
        "What does variance describe?",
        "Variance describes the expected squared deviation from a mean; sample estimators use the corresponding observed deviations.",
    ),
    (
        "Why can the median resist outliers better than the mean?",
        "The median depends on order rather than the magnitude of every observation, so extreme values can have less influence.",
    ),
    (
        "Why does correlation alone not establish causation?",
        "An association can arise from confounding, reverse causation, selection effects, or chance rather than a direct causal relationship.",
    ),
    (
        "What does a confidence level describe?",
        "A confidence level describes the long-run coverage of an interval-producing procedure under its assumptions.",
    ),
    (
        "What is a null hypothesis?",
        "A null hypothesis specifies a baseline statistical model or claim against which evidence is evaluated.",
    ),
    (
        "What is a p-value?",
        "A p-value is the probability, under the specified null model, of a test statistic at least as extreme as the observed one.",
    ),
    (
        "What is a Type I error?",
        "A Type I error rejects a null hypothesis that is true.",
    ),
    (
        "What is a Type II error?",
        "A Type II error fails to reject a null hypothesis when the relevant alternative is true.",
    ),
    (
        "What does conditional probability measure?",
        "Conditional probability measures the probability of an event given that another event has occurred.",
    ),
    (
        "What is sampling bias?",
        "Sampling bias occurs when the sampling process systematically misrepresents the population of interest.",
    ),
    (
        "What is a bootstrap sample?",
        "A bootstrap sample is drawn with replacement from the observed dataset and can help approximate an estimator's sampling distribution.",
    ),
];
