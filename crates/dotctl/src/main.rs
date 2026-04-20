use std::io;
use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Generator, Shell, generate};
use dotctl_core::ui::Ui;
use dotctl_core::{
    App, ApplyOptions, BootstrapOptions, DiffOptions, DoctorOptions, StateShowTarget, UpdateOptions,
};
use miette::Result;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "dotctl")]
#[command(about = "Workstation bootstrap and update CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Bootstrap(BootstrapArgs),
    Update(UpdateArgs),
    Diff,
    Apply,
    Doctor,
    State(StateArgs),
    Features(FeaturesArgs),
    Completion(CompletionArgs),
}

#[derive(Parser, Debug, Default)]
struct BootstrapArgs {
    #[arg(long)]
    repo: Option<PathBuf>,
    #[arg(long)]
    profile: Option<String>,
    #[arg(long = "package-manager")]
    package_manager: Option<String>,
    #[arg(long)]
    with_github: bool,
    #[arg(long)]
    with_terminal_apps: bool,
    #[arg(long)]
    with_git_lfs: bool,
    #[arg(long)]
    with_ai_tools: bool,
    #[arg(long)]
    with_fastfetch: bool,
    #[arg(long)]
    git_name: Option<String>,
    #[arg(long)]
    git_email: Option<String>,
    #[arg(long)]
    git_signing_key: Option<String>,
    #[arg(long)]
    git_gpg_program: Option<String>,
    #[arg(long)]
    sign_commits: bool,
    #[arg(long)]
    no_check: bool,
    #[arg(long)]
    no_prompt: bool,
    #[arg(long)]
    yes: bool,
}

#[derive(Parser, Debug, Default)]
struct UpdateArgs {
    #[arg(long)]
    no_check: bool,
}

#[derive(Parser, Debug)]
struct StateArgs {
    #[command(subcommand)]
    command: StateCommand,
}

#[derive(Subcommand, Debug)]
enum StateCommand {
    Show(StateShowArgs),
    Edit,
}

#[derive(Parser, Debug)]
struct StateShowArgs {
    #[arg(long, value_enum, default_value_t = ShowTarget::Local)]
    target: ShowTarget,
}

#[derive(Clone, Copy, Debug, ValueEnum, Default)]
enum ShowTarget {
    #[default]
    Local,
    Snapshot,
}

#[derive(Parser, Debug)]
struct FeaturesArgs {
    #[command(subcommand)]
    command: Option<FeaturesCommand>,
}

#[derive(Subcommand, Debug)]
enum FeaturesCommand {
    List,
}

#[derive(Parser, Debug)]
struct CompletionArgs {
    shell: Shell,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    let cli = Cli::parse();
    let app = App::new()?;

    match cli.command {
        Commands::Bootstrap(args) => app.bootstrap(BootstrapOptions {
            repo: args.repo,
            profile: args.profile,
            package_manager: args.package_manager,
            with_github: args.with_github,
            with_terminal_apps: args.with_terminal_apps,
            with_git_lfs: args.with_git_lfs,
            with_ai_tools: args.with_ai_tools,
            with_fastfetch: args.with_fastfetch,
            git_name: args.git_name,
            git_email: args.git_email,
            git_signing_key: args.git_signing_key,
            git_gpg_program: args.git_gpg_program,
            sign_commits: args.sign_commits,
            no_check: args.no_check,
            no_prompt: args.no_prompt || args.yes,
        }),
        Commands::Update(args) => app.update(UpdateOptions {
            no_check: args.no_check,
        }),
        Commands::Diff => match app.diff(DiffOptions::default())? {
            dotctl_core::DiffOutcome::Clean => Ok(()),
            dotctl_core::DiffOutcome::Dirty => std::process::exit(1),
        },
        Commands::Apply => app.apply(ApplyOptions::default()),
        Commands::Doctor => match app.doctor(DoctorOptions::default())? {
            dotctl_core::DoctorOutcome::Healthy => Ok(()),
            dotctl_core::DoctorOutcome::Unhealthy => std::process::exit(1),
        },
        Commands::State(state) => match state.command {
            StateCommand::Show(args) => {
                let target = match args.target {
                    ShowTarget::Local => StateShowTarget::Local,
                    ShowTarget::Snapshot => StateShowTarget::Snapshot,
                };
                print!("{}", app.show_state(target)?);
                Ok(())
            }
            StateCommand::Edit => app.edit_state(),
        },
        Commands::Features(args) => {
            match args.command.unwrap_or(FeaturesCommand::List) {
                FeaturesCommand::List => {
                    let ui = Ui::detect();
                    println!("{}", ui.section("feature flags"));
                    for (name, description) in app.features_list() {
                        println!("{}", ui.list_item(name, description));
                    }
                }
            }
            Ok(())
        }
        Commands::Completion(args) => {
            let mut command = Cli::command();
            print_completion(args.shell, &mut command);
            Ok(())
        }
    }
}

fn print_completion<G: Generator>(generator: G, command: &mut clap::Command) {
    generate(
        generator,
        command,
        command.get_name().to_string(),
        &mut io::stdout(),
    );
}
