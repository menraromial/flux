//! Flux Programming Language Compiler CLI
//! 
//! Command-line interface for the Flux compiler

use clap::Parser;
use std::process;
use std::path::Path;

use flux_compiler::cli::{Cli, Commands, CliContext, CompilerDriver};
use flux_compiler::tools::{Formatter, Linter, TestRunner, LintSeverity};
use flux_compiler::error::FluxResult;

fn main() {
    let cli = Cli::parse();
    
    // Change to specified directory if provided
    if let Some(dir) = &cli.directory {
        if let Err(e) = std::env::set_current_dir(dir) {
            eprintln!("error: Failed to change directory to {:?}: {}", dir, e);
            process::exit(1);
        }
    }

    let context = CliContext::new(cli.verbose, cli.quiet);
    
    let result = match &cli.command {
        Commands::Build { mode, target, output, optimize, check, progress, path } => {
            let driver = CompilerDriver::new(context.clone());
            // Create a temporary build args structure
            let build_command = Commands::Build {
                mode: mode.clone(),
                target: target.clone(),
                output: output.clone(),
                optimize: *optimize,
                check: *check,
                progress: *progress,
                path: path.clone(),
            };
            driver.build(&build_command)
        }
        
        Commands::Run { mode, args, path } => {
            let driver = CompilerDriver::new(context.clone());
            let run_command = Commands::Run {
                mode: mode.clone(),
                args: args.clone(),
                path: path.clone(),
            };
            driver.run(&run_command)
        }
        
        Commands::Test { filter, nocapture, jobs, path } => {
            let driver = CompilerDriver::new(context.clone());
            let test_command = Commands::Test {
                filter: filter.clone(),
                nocapture: *nocapture,
                jobs: *jobs,
                path: path.clone(),
            };
            driver.test(&test_command)
        }
        
        Commands::Fmt { check, files } => {
            handle_format_command(&context, *check, files)
        }
        
        Commands::Lint { fix, errors_only, files } => {
            handle_lint_command(&context, *fix, *errors_only, files)
        }
        
        Commands::Bench { filter, iterations, path } => {
            handle_bench_command(&context, filter.as_deref(), *iterations, path)
        }
        
        Commands::Doc { open, private, output, path } => {
            handle_doc_command(&context, *open, *private, output.as_deref(), path)
        }
        
        Commands::New { name, template, path } => {
            handle_new_command(&context, name, template, path)
        }
        
        Commands::Init { name, template, path } => {
            handle_init_command(&context, name.as_deref(), template, path)
        }
    };

    if let Err(e) = result {
        context.error(&format!("{}", e));
        process::exit(1);
    }
}

fn handle_format_command(context: &CliContext, check: bool, files: &[std::path::PathBuf]) -> FluxResult<()> {
    let formatter = Formatter::new(context.clone());
    
    if files.is_empty() {
        // Format all .flux files in current directory
        let results = formatter.format_directory(Path::new("."), check)?;
        
        if check {
            if results.changed > 0 {
                context.info(&format!("{} files need formatting", results.changed));
                process::exit(1);
            } else {
                context.success("All files are properly formatted");
            }
        } else {
            if results.changed > 0 {
                context.success(&format!("Formatted {} files", results.changed));
            } else {
                context.info("No files needed formatting");
            }
        }
        
        // Report errors
        for (path, error) in &results.errors {
            context.error(&format!("Failed to format {:?}: {}", path, error));
        }
    } else {
        // Format specific files
        let mut total_changed = 0;
        for file in files {
            match formatter.format_file(file, check) {
                Ok(changed) => {
                    if changed {
                        total_changed += 1;
                    }
                }
                Err(e) => {
                    context.error(&format!("Failed to format {:?}: {}", file, e));
                }
            }
        }
        
        if check && total_changed > 0 {
            context.info(&format!("{} files need formatting", total_changed));
            process::exit(1);
        } else if !check && total_changed > 0 {
            context.success(&format!("Formatted {} files", total_changed));
        }
    }
    
    Ok(())
}

fn handle_lint_command(context: &CliContext, fix: bool, errors_only: bool, files: &[std::path::PathBuf]) -> FluxResult<()> {
    let linter = Linter::new(context.clone());
    
    let files_to_lint = if files.is_empty() {
        // Find all .flux files in current directory
        find_flux_files(Path::new("."))?
    } else {
        files.to_vec()
    };
    
    let mut total_issues = 0;
    let mut error_count = 0;
    let mut warning_count = 0;
    
    for file in &files_to_lint {
        match linter.lint_file(file) {
            Ok(issues) => {
                for issue in &issues {
                    if errors_only && issue.severity != LintSeverity::Error {
                        continue;
                    }
                    
                    total_issues += 1;
                    match issue.severity {
                        LintSeverity::Error => error_count += 1,
                        LintSeverity::Warning => warning_count += 1,
                        LintSeverity::Info => {}
                    }
                    
                    let severity_str = match issue.severity {
                        LintSeverity::Error => "error".to_string().red(),
                        LintSeverity::Warning => "warning".to_string().yellow(),
                        LintSeverity::Info => "info".to_string().blue(),
                    };
                    
                    println!("{}:{}:{}: {}: {} [{}]", 
                        issue.file.display(),
                        issue.line,
                        issue.column,
                        severity_str,
                        issue.message,
                        issue.rule
                    );
                    
                    if let Some(suggestion) = &issue.suggestion {
                        println!("  help: {}", suggestion.dimmed());
                    }
                }
            }
            Err(e) => {
                context.error(&format!("Failed to lint {:?}: {}", file, e));
            }
        }
    }
    
    if total_issues == 0 {
        context.success("No linting issues found");
    } else {
        let summary = if errors_only {
            format!("Found {} errors", error_count)
        } else {
            format!("Found {} issues ({} errors, {} warnings)", 
                total_issues, error_count, warning_count)
        };
        
        if error_count > 0 {
            context.error(&summary);
            process::exit(1);
        } else {
            context.warn(&summary);
        }
    }
    
    Ok(())
}

fn handle_bench_command(context: &CliContext, filter: Option<&str>, iterations: usize, path: &Path) -> FluxResult<()> {
    context.info(&format!("Running benchmarks with {} iterations...", iterations));
    
    // TODO: Implement actual benchmark runner
    context.info("Benchmark runner not yet implemented");
    
    Ok(())
}

fn handle_doc_command(context: &CliContext, open: bool, private: bool, output: Option<&Path>, path: &Path) -> FluxResult<()> {
    context.info("Generating documentation...");
    
    // TODO: Implement documentation generator
    context.info("Documentation generator not yet implemented");
    
    Ok(())
}

fn handle_new_command(context: &CliContext, name: &str, template: &flux_compiler::cli::ProjectTemplate, path: &Path) -> FluxResult<()> {
    context.info(&format!("Creating new project '{}' with template {:?}", name, template));
    
    // TODO: Implement project creation
    context.info("Project creation not yet implemented");
    
    Ok(())
}

fn handle_init_command(context: &CliContext, name: Option<&str>, template: &flux_compiler::cli::ProjectTemplate, path: &Path) -> FluxResult<()> {
    let project_name = name.unwrap_or_else(|| {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("flux-project")
    });
    
    context.info(&format!("Initializing project '{}' with template {:?}", project_name, template));
    
    // TODO: Implement project initialization
    context.info("Project initialization not yet implemented");
    
    Ok(())
}

fn find_flux_files(dir: &Path) -> FluxResult<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).map_err(|e| flux_compiler::FluxError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| flux_compiler::FluxError::Io(e.to_string()))?;
            let path = entry.path();
            
            if path.is_dir() {
                files.extend(find_flux_files(&path)?);
            } else if path.extension().and_then(|s| s.to_str()) == Some("flux") {
                files.push(path);
            }
        }
    }
    
    Ok(files)
}

// Import colored trait for string coloring
use colored::*;