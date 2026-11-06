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

use alloc::vec;
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub use context::TaskContext;
use log::{info, trace};

use crate::{
    loader, sbi,
    sync::UPSafeCell,
    task::{
        switch::__switch,
        task_inner::{TaskControlBlock, TaskStatus},
    },
    trap::TrapContext,
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

/// The task manager inner in [`UPSafeCell`].
struct TaskManagerInner {
    /// Task list.
    tasks: Vec<TaskControlBlock>,
    /// Id of current [`TaskStatus::Running`] task.
    current_task: usize,
}

lazy_static! {
    /// A [`TaskManager`] global instance through [`lazy_static!`].
    pub static ref TASK_MANAGER: TaskManager = {
        trace!("init TASK_MANAGER");
        let num_app = loader::get_num_app();
        trace!("num_app = {num_app}");
        let mut tasks = vec![];
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(loader::get_app_data(i), i));
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
    /// But for now, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &raw const next_task.task_cx;
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

    /// Find next task to run and return task id.
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

    /// Get the current [`TaskStatus::Running`] task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current [`TaskStatus::Running`] task's [`TrapContext`]s.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current [`TaskStatus::Running`] task's program break.
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].change_program_brk(size)
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

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current [`TaskStatus::Running`] task to the task we have found, or
/// there is no [`TaskStatus::Ready`] task and we can exit with all
/// applications completed.
pub fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current [`TaskStatus::Running`] task into
/// [`TaskStatus::Ready`].
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current [`TaskStatus::Running`] task into
/// [`TaskStatus::Exited`].
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current [`TaskStatus::Running`] task and run the next task in
/// task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current [`TaskStatus::Running`] task and run the next task in task
/// list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current [`TaskStatus::Running`] task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current [`TaskStatus::Running`] task's [`TrapContext`]s.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current [`TaskStatus::Running`] task's program break.
pub fn change_current_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}
