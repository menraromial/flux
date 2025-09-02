//! Command-line interface for the Flux compiler
//! 
//! This module provides the CLI commands and argument parsing for the Flux compiler,
//! including build, run, test, format, and lint commands.

use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Instant;

use crate::error::FluxResult;
use crate::package::{ProjectInstance, BuildConfig, OptimizationLevel};

/// Flux Programming Language Compiler
#[derive(Parser)]
#[command(name = "flux")]
#[command(about = "A modern programming language compiler")]
#[command(version = "0.1.0")]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Set the working directory
    #[arg(short = 'C', long, global = true)]
    pub directory: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compile Flux source files
    Build {
        /// Build mode
        #[arg(short, long, default_value = "debug")]
        mode: BuildMode,

        /// Target architecture
        #[arg(short, long)]
        target: Option<CompilationTarget>,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Enable optimization
        #[arg(short = 'O', long)]
        optimize: bool,

        /// Check syntax only (no code generation)
        #[arg(long)]
        check: bool,

        /// Show compilation progress
        #[arg(long)]
        progress: bool,

        /// Source files or project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Compile and run Flux program
    Run {
        /// Build mode
        #[arg(short, long, default_value = "debug")]
        mode: BuildMode,

        /// Arguments to pass to the program
        #[arg(last = true)]
        args: Vec<String>,

        /// Source files or project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Run tests
    Test {
        /// Test name pattern to filter
        #[arg(short, long)]
        filter: Option<String>,

        /// Show test output
        #[arg(long)]
        nocapture: bool,

        /// Number of test threads
        #[arg(short, long)]
        jobs: Option<usize>,

        /// Source files or project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Format source code
    Fmt {
        /// Check if files are formatted without modifying them
        #[arg(long)]
        check: bool,

        /// Files to format (default: all .flux files)
        files: Vec<PathBuf>,
    },

    /// Lint source code
    Lint {
        /// Fix automatically fixable issues
        #[arg(long)]
        fix: bool,

        /// Show only errors, not warnings
        #[arg(long)]
        errors_only: bool,

        /// Files to lint (default: all .flux files)
        files: Vec<PathBuf>,
    },

    /// Run benchmarks
    Bench {
        /// Benchmark name pattern to filter
        #[arg(short, long)]
        filter: Option<String>,

        /// Number of benchmark iterations
        #[arg(long, default_value = "100")]
        iterations: usize,

        /// Source files or project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Generate documentation
    Doc {
        /// Open documentation in browser
        #[arg(long)]
        open: bool,

        /// Include private items
        #[arg(long)]
        private: bool,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Source files or project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Create a new Flux project
    New {
        /// Project name
        name: String,

        /// Project template
        #[arg(long, default_value = "binary")]
        template: ProjectTemplate,

        /// Target directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Initialize a Flux project in existing directory
    Init {
        /// Project name (default: directory name)
        name: Option<String>,

        /// Project template
        #[arg(long, default_value = "binary")]
        template: ProjectTemplate,

        /// Target directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum BuildMode {
    /// Debug build with debug symbols
    Debug,
    /// Release build with optimizations
    Release,
    /// Check syntax and types only
    Check,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ProjectTemplate {
    /// Binary application
    Binary,
    /// Library crate
    Library,
    /// Web application
    Web,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CompilationTarget {
    /// Native binary (default)
    Native,
    /// WebAssembly module
    Wasm,
    /// WebAssembly with JavaScript bindings
    WasmJs,
}

/// CLI execution context
#[derive(Clone)]
pub struct CliContext {
    pub verbose: bool,
    pub quiet: bool,
    pub start_time: Instant,
}

impl CliContext {
    pub fn new(verbose: bool, quiet: bool) -> Self {
        Self {
            verbose,
            quiet,
            start_time: Instant::now(),
        }
    }

    /// Print info message if not quiet
    pub fn info(&self, message: &str) {
        if !self.quiet {
            println!("{}", message);
        }
    }

    /// Print verbose message if verbose mode enabled
    pub fn verbose(&self, message: &str) {
        if self.verbose && !self.quiet {
            println!("{} {}", "verbose:".dimmed(), message.dimmed());
        }
    }

    /// Print warning message
    pub fn warn(&self, message: &str) {
        if !self.quiet {
            eprintln!("{} {}", "warning:".yellow().bold(), message);
        }
    }

    /// Print error message
    pub fn error(&self, message: &str) {
        eprintln!("{} {}", "error:".red().bold(), message);
    }

    /// Print success message
    pub fn success(&self, message: &str) {
        if !self.quiet {
            println!("{} {}", "success:".green().bold(), message);
        }
    }

    /// Create a progress bar
    pub fn progress_bar(&self, len: u64, message: &str) -> Option<ProgressBar> {
        if self.quiet || !self.verbose {
            return None;
        }

        let pb = ProgressBar::new(len);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(message.to_string());
        Some(pb)
    }

    /// Get elapsed time since CLI started
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

/// Compiler driver that coordinates all compilation phases
pub struct CompilerDriver {
    context: CliContext,
}

impl CompilerDriver {
    pub fn new(context: CliContext) -> Self {
        Self { context }
    }

    /// Execute build command
    pub fn build(&self, command: &Commands) -> FluxResult<()> {
        if let Commands::Build { mode, target, output, optimize, check, progress, path } = command {
            self.context.verbose(&format!("Building project at {:?}", path));
            self.context.verbose(&format!("Build mode: {:?}", mode));

            // Load project configuration
            let project = ProjectInstance::load(path)?;
            self.context.verbose(&format!("Loaded project: {}", project.name()));

            // Create build configuration
            let mut build_config = match mode {
                BuildMode::Debug => BuildConfig::debug(),
                BuildMode::Release => BuildConfig::release(),
                BuildMode::Check => {
                    let mut config = BuildConfig::debug();
                    config.optimization_level = OptimizationLevel::None;
                    config
                }
            };

            if let Some(target) = target {
                build_config.target = Some(match target {
                    CompilationTarget::Native => "native".to_string(),
                    CompilationTarget::Wasm => "wasm32-unknown-unknown".to_string(),
                    CompilationTarget::WasmJs => "wasm32-unknown-unknown".to_string(),
                });
                
                // Set WebAssembly-specific configuration
                if matches!(target, CompilationTarget::Wasm | CompilationTarget::WasmJs) {
                    build_config.wasm_target = true;
                    build_config.generate_js_bindings = matches!(target, CompilationTarget::WasmJs);
                }
            }

            if let Some(output) = output {
                build_config.output_dir = output.clone();
            }

            if *optimize {
                build_config.optimization_level = OptimizationLevel::Speed;
            }

            build_config.verbose = self.context.verbose;

            // Show progress if requested
            let progress_bar = if *progress {
                self.context.progress_bar(5, "Compiling")
            } else {
                None
            };

            // Phase 1: Lexical analysis
            if let Some(pb) = &progress_bar {
                pb.set_message("Lexical analysis");
                pb.inc(1);
            }
            self.context.verbose("Running lexical analysis...");

            // Phase 2: Parsing
            if let Some(pb) = &progress_bar {
                pb.set_message("Parsing");
                pb.inc(1);
            }
            self.context.verbose("Parsing source files...");

            // Phase 3: Semantic analysis
            if let Some(pb) = &progress_bar {
                pb.set_message("Semantic analysis");
                pb.inc(1);
            }
            self.context.verbose("Running semantic analysis...");

            // Phase 4: Code generation (skip if check-only)
            if !check {
                if let Some(pb) = &progress_bar {
                    pb.set_message("Code generation");
                    pb.inc(1);
                }
                self.context.verbose("Generating code...");
            }

            // Phase 5: Linking
            if !check {
                if let Some(pb) = &progress_bar {
                    pb.set_message("Linking");
                    pb.inc(1);
                }
                self.context.verbose("Linking executable...");
            }

            if let Some(pb) = &progress_bar {
                pb.finish_with_message("Compilation complete");
            }

            // Build the project
            project.build(&build_config)?;

            let elapsed = self.context.elapsed();
            if *check {
                self.context.success(&format!("Check completed in {:.2}s", elapsed.as_secs_f64()));
            } else {
                self.context.success(&format!("Build completed in {:.2}s", elapsed.as_secs_f64()));
            }

            Ok(())
        } else {
            Err(crate::error::FluxError::Cli("Invalid command for build".to_string()))
        }
    }

    /// Execute run command
    pub fn run(&self, command: &Commands) -> FluxResult<()> {
        if let Commands::Run { mode, args, path } = command {
            self.context.verbose(&format!("Running project at {:?}", path));

            // First build the project
            let build_command = Commands::Build {
                mode: mode.clone(),
                target: None,
                output: None,
                optimize: false,
                check: false,
                progress: false,
                path: path.clone(),
            };

            self.build(&build_command)?;

            // Then execute it
            self.context.info("Running executable...");
            
            // Load project to get executable path
            let project = ProjectInstance::load(path)?;
            project.run(args)?;

            Ok(())
        } else {
            Err(crate::error::FluxError::Cli("Invalid command for run".to_string()))
        }
    }

    /// Execute test command
    pub fn test(&self, command: &Commands) -> FluxResult<()> {
        if let Commands::Test { filter, nocapture, jobs, path } = command {
            self.context.verbose(&format!("Running tests for project at {:?}", path));

            let project = ProjectInstance::load(path)?;
            
            // Build test configuration
            let mut build_config = BuildConfig::debug();
            build_config.verbose = self.context.verbose;
            
            if let Some(jobs) = jobs {
                build_config.parallel = *jobs > 1;
            }

            // Run tests
            let test_results = project.test(&build_config, filter.as_deref())?;

            // Report results
            let passed = test_results.passed;
            let failed = test_results.failed;
            let total = passed + failed;

            if failed == 0 {
                self.context.success(&format!("All {} tests passed", total));
            } else {
                self.context.error(&format!("{} of {} tests failed", failed, total));
            }

            Ok(())
        } else {
            Err(crate::error::FluxError::Cli("Invalid command for test".to_string()))
        }
    }
}

