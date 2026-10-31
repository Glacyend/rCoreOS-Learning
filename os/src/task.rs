//! Task management implementation.
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called [`TASK_MANAGER`]
//! controls all the tasks in the operating system.
//!
//! Be careful when you see [`__switch`] ASM function in `switch.S`. Control
//! flow around this function might not be what you expect.

mod context;
mod switch;
mod task_inner;

use lazy_static::lazy_static;

pub use context::TaskContext;
use log::info;

use crate::{
    config::MAX_APP_NUM,
    loader, sbi,
    sync::UPSafeCell,
    task::{
        switch::__switch,
        task_inner::{TaskControlBlock, TaskStatus},
    },
};

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on [`TaskManager`] deals with all task state
/// transitions and task context switching. For convenience, you can find
/// wrappers around it in the module level.
///
/// Most of [`TaskManager`] are hidden behind the field [`TaskManager::inner`],
/// to defer borrowing checks to runtime. You can see examples on how to use
/// [`TaskManager::inner`] in existing functions on [`TaskManager`].
pub struct TaskManager {
    /// Total number of tasks.
    num_app: usize,
    /// Use inner value to get mutable access.
    inner: UPSafeCell<TaskManagerInner>,
}

/// Inner of [`TaskManager`].
pub struct TaskManagerInner {
    /// Task list.
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// Id of current [`TaskStatus::Running`] task.
    current_task: usize,
}

lazy_static! {
    /// Global variable: [`TASK_MANAGER`]
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = loader::get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(loader::init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            }
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero
    /// process later).
    /// But in the future, we load apps statically, so the first task is a real
    /// app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &raw const task0.task_cx;
        drop(inner);
        let mut unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped
        // manually.
        unsafe {
            __switch(&raw mut unused, next_task_cx_ptr);
        }
        unreachable!();
    }

    /// Change the status of current [`TaskStatus::Running`] task into
    /// [`TaskStatus::Ready`].
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// Change the status of current [`TaskStatus::Running`] task into
    /// [`TaskStatus::Exited`].
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return app id.
    ///
    /// In this case, we only return the first [`TaskStatus::Ready`] task in
    /// the task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        ((current + 1)..(current + self.num_app + 1))
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch current [`TaskStatus::Ready`] task to the task we have found, or
    /// there is no [`TaskStatus::Ready`] task and we can exit with all
    /// applications completed.
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &raw mut inner.tasks[current].task_cx;
            let next_task_cx_ptr = &raw const inner.tasks[next].task_cx;
            drop(inner);
            // before this, we should drop local variables that must be dropped
            // manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            info!("[kernel] App applications completed!");
            sbi::shutdown(false);
        }
    }
}

/// Run first task.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Run next task.
pub fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Suspend current task.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Exit current task.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend current task, then run next task.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit current task, then run next task.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
