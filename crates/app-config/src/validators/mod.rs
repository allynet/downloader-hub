use validator::{ValidationErrors, ValidationErrorsKind};

pub mod directory;
pub mod file;
pub mod str;
pub mod url;

pub fn print_validation_errors(e: &ValidationErrors, prefix: &str, level: usize) {
    let level = level.max(1);
    for (e_name, e) in e.errors() {
        match e {
            ValidationErrorsKind::Field(e) => {
                let prefix_rep = prefix.repeat(level);
                eprintln!(
                    "{prefix_rep}{e_name}:\n{}",
                    e.iter()
                        .map(|x| format!("{} {:?}", x.code, x.params))
                        .fold(String::new(), |acc, a| format!(
                            "{acc}{prefix_rep}{prefix}- {a}\n"
                        ))
                        .trim_end()
                );
            }

            ValidationErrorsKind::Struct(e) => {
                eprintln!("{}{}:", prefix, e_name);
                print_validation_errors(e, prefix, level + 1);
            }

            ValidationErrorsKind::List(e) => {
                eprintln!("{}{}:", prefix, e_name);
                for e in e.values() {
                    print_validation_errors(e, prefix, level + 1);
                }
            }
        }
    }
}
