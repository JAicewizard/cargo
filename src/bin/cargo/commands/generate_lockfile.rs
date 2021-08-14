use crate::command_prelude::*;

use cargo::{core::compiler::CompileKind, ops};

pub fn cli() -> App {
    subcommand("generate-lockfile")
        .about("Generate the lockfile for a package")
        .arg(opt("quiet", "No output printed to stdout").short("q"))
        .arg_manifest_path()
        .after_help("Run `cargo help generate-lockfile` for more detailed information.\n")
}

pub fn exec(config: &mut Config, args: &ArgMatches<'_>) -> CliResult {
    let ws = args.workspace(config)?;

    let targets = if args.is_present("all-targets") {
        config
            .shell()
            .warn("the --all-targets flag has been changed to --target=all")?;
        vec!["all".to_string()]
    } else {
        args._values_of("target")
    };

    let requested_kinds = CompileKind::from_requested_targets(ws.config(), &*targets)?;

    ops::generate_lockfile(&ws, &*requested_kinds)?;
    Ok(())
}
