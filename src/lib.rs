pub mod note;
pub mod parser;
pub mod status;
#[cfg(windows)]
pub mod win;

#[cfg(windows)]
pub unsafe fn output() {
    win::output()
}
