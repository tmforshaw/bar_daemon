use crate::error::DaemonError;

/// # Errors
/// Returns an error if the command for requested value cannot be spawned
/// Returns an error if values in the output of the command cannot found
/// Returns an error if output cannot be converted to String
pub fn run<S: AsRef<str>>(name: S, args: &[S]) -> Result<String, DaemonError> {
    // Run the command, changing any errors into CommandError with the name and args given as parameters
    let command_output = std::process::Command::new(name.as_ref())
        .args(args.iter().map(AsRef::as_ref))
        .output()
        .map_err(|e| DaemonError::CommandError {
            name: name.as_ref().to_string(),
            args: args.iter().map(AsRef::as_ref).map(ToString::to_string).collect::<Vec<_>>(),
            e: e.to_string(),
        })?;

    Ok(String::from_utf8(command_output.stdout)?.trim().to_string())
}
