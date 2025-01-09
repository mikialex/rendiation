use std::path::PathBuf;

use itertools::Itertools;

use crate::ConsoleWindow;

impl ConsoleWindow {
  pub(crate) fn tab_complete(&mut self) {
    let last = self.get_last_line().to_string();

    let args = ConsoleWindow::digest_line(&last);
    if args.is_empty() {
      return;
    }
    let last_arg = &args[args.len() - 1];
    let is_command_arg = args.len() == 1;
    let mut quote_char = self.tab_quote;
    if self.tab_string.is_empty() {
      // means we are entering tab search mode

      // the main fiddling here is for path with spaces in them on mac and windows
      // the user can enter a partial path with quotes, which we have to strip before passing to the
      // fs tabber and reinstate after an answer comes back
      // else if we get a path with spaces in it from the fs tabber we have to add quotes

      self.tab_quoted = false;

      if last_arg.is_empty() {
        return;
      }

      if &last_arg[0..1] == "\'" || &last_arg[0..1] == "\"" {
        self.tab_string = last_arg[1..].to_string();
        quote_char = last_arg.chars().next().unwrap();
      } else {
        self.tab_string = last_arg.to_string()
      };
      self.tab_nth = 0;
      self.tab_offset = self.text.len() - last_arg.len();
    } else {
      // otherwise move to the next match
      self.tab_nth += 1;
    }
    // the loop gets us back to the first match once fs tabber returns no match
    loop {
      if let Some(mut path) = if is_command_arg {
        cmd_tab_complete(&self.tab_string, self.tab_nth, &self.tab_command_table)
      } else {
        fs_tab_complete(&self.tab_string, self.tab_nth)
      } {
        let mut added_quotes = false;

        if path.display().to_string().contains(' ') {
          path = PathBuf::from(format!("{}{}{}", quote_char, path.display(), quote_char));
          added_quotes = true;
        }

        self.text.truncate(self.tab_offset);
        self.force_cursor_to_end = true;
        self.text.push_str(path.to_str().unwrap());

        self.tab_quoted = added_quotes;
        break;
      } else {
        // exit if there were no matches at all
        if self.tab_nth == 0 {
          break;
        }
        // force wrap around to first match
        self.tab_nth = 0;
      }
    }
  }
  // chop up input line input arguments honoring quotes

  fn digest_line(line: &str) -> Vec<&str> {
    enum State {
      InQuotes(char),
      InWhite,
      InWord,
      NotSure,
    }

    let mut state = State::InWord;

    let mut res: Vec<&str> = Vec::new();
    let mut start = 0;

    for (idx, ch) in line.char_indices() {
      match state {
        State::InWord => match ch {
          ' ' => {
            res.push(&line[start..idx]);
            state = State::InWhite;
          }
          '"' | '\'' => {
            state = State::InQuotes(ch);
            start = idx;
          }
          _ => {}
        },
        State::InWhite => match ch {
          ' ' => {}
          '"' | '\'' => {
            state = State::InQuotes(ch);
            start = idx;
          }
          _ => {
            start = idx;
            state = State::InWord;
          }
        },
        State::InQuotes(qc) => {
          if ch == qc {
            res.push(&line[start..idx + 1]);
            state = State::NotSure;
            start = idx;
          }
        }
        State::NotSure => {
          if ch == ' ' {
            state = State::InWhite;
          } else {
            state = State::InWord;
          }
          start = idx;
        }
      }
    }

    match state {
      State::InWord => res.push(&line[start..]),
      State::InWhite => res.push(""),
      State::InQuotes(_) => res.push(&line[start..]),
      State::NotSure => {}
    }

    res
  }
}
pub(crate) fn cmd_tab_complete(search: &str, nth: usize, commands: &[String]) -> Option<PathBuf> {
  commands
    .iter()
    .filter(|c| c.starts_with(search))
    .nth(nth)
    .map(PathBuf::from)
  // None
}
// return the nth matching path, or None if there isnt one
pub(crate) fn fs_tab_complete(search: &str, nth: usize) -> Option<PathBuf> {
  let dot_slash = if cfg!(target_os = "windows") && search.find('\\').is_some() {
    ".\\"
  } else {
    "./"
  };
  let search_path = PathBuf::from(search);

  let mut nth = nth;
  let mut added_dot = false;

  // were we given a real path to start with?

  let mut base_search = if search_path.is_dir() {
    search_path
  } else {
    // no - look at the parent (ie we got "cd dir/f")
    let parent = search_path.parent();
    if parent.is_none() {
      return None;
    } else {
      let p = parent.unwrap().to_path_buf();
      // if empty parent then search "." (remember we added the dot so remove it later)
      if p.display().to_string().is_empty() {
        added_dot = true;
        PathBuf::from(dot_slash)
      } else if p.display().to_string() == "." {
        // we were given . as a dir
        PathBuf::from(dot_slash)
      } else {
        p
      }
    }
  };
  // convert .. to ../ or ..\
  if base_search.display().to_string() == ".." {
    base_search = PathBuf::from(format!(".{}", dot_slash));
  }

  let dir = std::fs::read_dir(&base_search);

  if let Ok(entries) = dir {
    // deal with platform oddities, also unwrap everything

    // mac retruns things in random order - so sort
    #[cfg(target_os = "macos")]
    let entries = entries
      .flatten()
      .sorted_by(|a, b| Ord::cmp(&a.file_name(), &b.file_name()));

    // windows returns things in ascii case sensitive order
    // this is a surprise to windows users
    #[cfg(target_os = "windows")]
    let entries = entries.flatten().sorted_by(|a, b| {
      Ord::cmp(
        &a.file_name().to_ascii_lowercase(),
        &b.file_name().to_ascii_lowercase(),
      )
    });

    // linux is well behaved!
    #[cfg(target_os = "linux")]
    let entries = entries.filter(|e| e.is_ok()).map(|e| e.unwrap());

    for ent in entries {
      let mut ret_path = ent.path();
      if added_dot {
        ret_path = ret_path.strip_prefix(dot_slash).ok()?.to_path_buf();
      }
      #[cfg(target_os = "windows")]
      if ret_path
        .display()
        .to_string()
        .to_ascii_lowercase()
        .starts_with(search)
      {
        if nth == 0 {
          return Some(ret_path);
        } else {
          nth -= 1;
        }
      }
      #[cfg(not(target_os = "windows"))]
      if ret_path.display().to_string().starts_with(search) {
        if nth == 0 {
          return Some(ret_path);
        } else {
          nth -= 1;
        }
      }
    }
  }
  None
}

#[test]
fn test_digest_line() {
  // let mut console = ConsoleWindow::new(">> ");
  let result = ConsoleWindow::digest_line("cd foo");
  assert_eq!(result, vec!["cd", "foo"]);
  let result = ConsoleWindow::digest_line("cd foo ");
  assert_eq!(result, vec!["cd", "foo", ""]);
  let result = ConsoleWindow::digest_line("cd \"foo bar\"");
  assert_eq!(result, vec!["cd", "\"foo bar\""]);
  let result = ConsoleWindow::digest_line("cd \"foo bar");
  assert_eq!(result, vec!["cd", "\"foo bar"]);
  // let result = console.digest_line("cd foo bar\"");
  // assert_eq!(result, vec!["cd", "foo", "bar\""]);
  // let result = console.digest_line("\"cd foo bar\"");
  // assert_eq!(result, vec!["\"cd", "foo", "bar\""]);
  // let result = console.digest_line("cd\" foo bar\"");
  // assert_eq!(result, vec!["cd\"", "foo", "bar\""]);
}
