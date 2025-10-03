mod common;
mod processor;
pub mod task;

use std::sync::LazyLock;

use deadqueue::unlimited::Queue;
pub use processor::TaskQueueProcessor;
pub use task::Task;
use tracing::trace;

static TASK_QUEUE: LazyLock<Queue<Task>> = LazyLock::new(Queue::new);

pub struct TaskQueue;
impl TaskQueue {
    pub fn push(task: Task) {
        trace!(?task, "Pushing task to queue");
        TASK_QUEUE.push(task);
    }
}
