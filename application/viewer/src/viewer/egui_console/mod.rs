/// A console window for egui / eframe applications
///
/// [Egui / eframe ]: <https://github.com/emilk/egui>
///
/// # Example
///
/// You need a [`ConsoleWindow`] instance in your egui App
/// ```ignore
///pub struct ConsoleDemo {
///     ...
///    console: ConsoleWindow,
///}
/// ```
/// Then in the construction phase use [`ConsoleBuilder`] to create a new ConsoleWindow
/// ```ignore
/// impl Default for ConsoleDemo {
///    fn default() -> Self {
///       Self {
///          ...
///         console: ConsoleBuilder::new().prompt(">> ").history_size(20).build()
///      }
///    }
/// }
/// ```
///
/// Now in the egui update callback you must [`ConsoleWindow::draw`] the console in a host container, typically an egui Window
///
/// ```ignore
///  let mut console_response: ConsoleEvent = ConsoleEvent::None;
///  egui::Window::new("Console Window")
///      .default_height(500.0)
///      .resizable(true)
///      .show(ctx, |ui| {
///        console_response = self.console.draw(ui);
///  });
///```
///
/// The draw method returns a [`ConsoleEvent`] that you can use to respond to user input. If the user entered a command then you can hndle that command as you like.
/// The code here simply echos the command back to the user and reissues the prompt.
///
///```ignore
/// if let ConsoleEvent::Command(command) = console_response {
///    self.console.print(format!("You entered: {}", command));
///    self.console.prompt();
/// }
///
///```
///
///
///#  Command history
///
/// - ctrl-r searches the command history
/// - up and down arrow walk though the command history
///
/// If you want the command history to be automatically persisted you need to enable the persistence feature. This will use the eframe storage to save the command history between sessions.
///
/// Alternatively you can use [`ConsoleWindow::load_history`] and [`ConsoleWindow::get_history`] to manually save and load the command history.    
#[warn(missing_docs)]
pub mod console;
mod tab;
pub use crate::console::ConsoleBuilder;
pub use crate::console::ConsoleEvent;
pub use crate::console::ConsoleWindow;
