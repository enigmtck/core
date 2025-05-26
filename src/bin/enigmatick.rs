use anyhow::Result;
use chrono::{DateTime, Duration, Utc}; // MODIFIED LINE: Added DateTime
use clap::{Parser, Subcommand};
use comfy_table::{presets, Attribute, Cell, Color, Table, ColumnConstraint, Width}; // MODIFIED LINE: Added Width
use enigmatick::models::instances::{
    self as instance_model, Instance, SortField as LibSortField, SortDirection as LibSortDirection, SortParam as LibSortParam
};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use enigmatick::admin::create_user;
use enigmatick::models::actors::ActorType;
use enigmatick::{admin::NewUser, server};
use enigmatick::{POOL, SYSTEM_USER};
use rand::distributions::{Alphanumeric, DistString};
use rust_embed::RustEmbed;
use std::fs;
use std::io::stdout;
use tokio::runtime::Runtime;

// Helper struct for RAII raw mode management
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> std::io::Result<Self> {
        enable_raw_mode()?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if let Err(e) = disable_raw_mode() {
            // eprintln can be problematic in drop if terminal is already messed up,
            // but for a CLI app, it's often the best effort.
            eprintln!(
                "Error: Failed to disable raw mode: {e}. Terminal might be in an unexpected state."
            );
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations.pg");
    } else if #[cfg(feature = "sqlite")] {
        pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations.sqlite");
    }
}

#[derive(RustEmbed)]
#[folder = "bundled/"]
pub struct Bundled;

#[derive(Parser)]
pub struct CacheArgs {
    #[command(subcommand)]
    command: CacheCommands,
}

#[derive(Subcommand)]
pub enum CacheCommands {
    /// Prune cached files older than the specified duration (e.g., 30d, 2m, 1y)
    Prune { duration: String },
    /// Delete a specific cached item by its URL
    Delete { url: String },
}

// Helper function to parse a single sort field string like "blocked" or "blocked:asc"
fn parse_one_lib_sort_param(s: &str) -> Result<LibSortParam, String> {
    let parts: Vec<&str> = s.split(':').collect();
    
    let field_str = parts[0];
    let direction_str_opt = parts.get(1).copied(); // Use .copied() to get Option<&str>

    if parts.len() > 2 {
        return Err(format!("Invalid sort format: '{s}'. Expected 'field' or 'field:direction'. Too many colons."));
    }

    let field = match field_str.to_lowercase().as_str() {
        "domain" | "domain_name" | "name" => LibSortField::DomainName,
        "blocked" => LibSortField::Blocked,
        "last" | "last_message_at" | "lastmessageat" => LibSortField::LastMessageAt,
        _ => return Err(format!("Unknown sort field: '{}'", field_str)),
    };

    let direction = match direction_str_opt {
        Some(dir_str) => match dir_str.to_lowercase().as_str() {
            "asc" | "ascending" => LibSortDirection::Asc,
            "desc" | "descending" => LibSortDirection::Desc,
            _ => return Err(format!("Unknown sort direction: '{}'", dir_str)),
        },
        None => LibSortDirection::Asc, // Default to Ascending if no direction is specified
    };

    Ok(LibSortParam { field, direction })
}

// The parse_sort_string_to_lib_params function remains unchanged as it calls parse_one_lib_sort_param.
fn parse_sort_string_to_lib_params(s: &str) -> Result<Vec<LibSortParam>, String> {
    s.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(parse_one_lib_sort_param)
        .collect()
}


#[derive(Parser)]
pub struct InstanceArgs {
    #[command(subcommand)]
    command: InstanceCommands,
}

#[derive(Subcommand)]
pub enum InstanceCommands {
    /// List all instances with pagination
    List {
        #[clap(long, default_value = "1")]
        page: i64,
        #[clap(long)]
        page_size: Option<i64>,
        /// Sort order for the instance list.
        ///
        /// Format: "field[:direction][,field[:direction]...]"
        ///
        /// - 'field' can be one of:
        ///   - 'domain' (or 'name', 'domain_name')
        ///   - 'blocked'
        ///   - 'last' (or 'last_message_at')
        /// - 'direction' can be 'asc' (ascending) or 'desc' (descending).
        /// - If direction is omitted, 'asc' is assumed.
        /// - Multiple sort criteria can be comma-separated.
        ///
        /// Examples:
        ///   --sort blocked
        ///   --sort last:desc
        ///   --sort blocked:asc,domain:desc
        #[clap(long)]
        sort: Option<String>,
    },
    /// Block an instance by its domain name
    Block { domain_name: String },
    /// Unblock an instance by its domain name
    Unblock { domain_name: String },
    /// Get details for a specific instance by its domain name
    Get { domain_name: String },
}

#[derive(Parser)]
pub enum Commands {
    /// Initialize the necessary folder structure (e.g., for media).
    /// This should be run once before starting the server for the first time.
    Init,
    /// Generate a template .env file named '.env.template'.
    /// Copy this to '.env' and fill in your configuration values.
    Template,
    /// Run database migrations to set up or update the database schema.
    /// This is necessary before starting the server and after updates.
    Migrate,
    /// Manage cached media files.
    /// Use subcommands like 'prune' or 'delete'.
    Cache(CacheArgs),
    /// Create or ensure the system user exists in the database.
    /// The system user is used for server-to-server activities and internal tasks.
    SystemUser,
    /// Start the Enigmatick web server and background task runners.
    Server,
    /// Manage known instances (other federated servers).
    /// Allows listing, blocking, unblocking, and viewing details of instances.
    Instances(InstanceArgs),
}

#[derive(Parser)] // requires `derive` feature
#[command(name = "enigmatick")]
#[command(version = env!("CARGO_PKG_VERSION"))] // Add version from Cargo.toml
#[command(about = "Enigmatick: A federated communication platform server.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Init => handle_init().expect("init failed"),
        Commands::Template => handle_template().expect("template loading failed"),
        Commands::Migrate => handle_migrations().expect("migrate failed"),
        Commands::Cache(args) => handle_cache_command(args).expect("cache command failed"),
        Commands::SystemUser => handle_system_user().expect("failed to create system user"),
        Commands::Instances(args) => {
            handle_instance_command(args).expect("instance command failed")
        }
        Commands::Server => server::start(),
    }
}

fn handle_init() -> Result<()> {
    println!("creating folder structure...");
    fs::create_dir_all("media/avatars")?;
    fs::create_dir_all("media/banners")?;
    fs::create_dir_all("media/cache")?;
    fs::create_dir_all("media/uploads")?;
    println!("complete.");

    Ok(())
}

fn handle_template() -> Result<()> {
    if let Some(template) = Bundled::get(".env.template") {
        fs::write(".env.template", template.data)?;
    }
    Ok(())
}

fn handle_migrations() -> Result<()> {
    println!("running database migrations...");
    let conn = &mut POOL.get().expect("failed to retrieve database connection");

    conn.run_pending_migrations(MIGRATIONS)
        .map_err(anyhow::Error::msg)?;
    println!("complete.");

    Ok(())
}

fn parse_duration(duration_str: &str) -> Result<Duration> {
    let duration_str = duration_str.trim();
    let numeric_part = duration_str
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    let unit_part = duration_str
        .chars()
        .skip_while(|c| c.is_ascii_digit())
        .collect::<String>();

    let value = numeric_part.parse::<i64>()?;

    match unit_part.as_str() {
        "d" => Ok(Duration::days(value)),
        "m" => Ok(Duration::days(value * 30)), // Approximate months
        "y" => Ok(Duration::days(value * 365)), // Approximate years
        _ => Err(anyhow::anyhow!(
            "Invalid duration unit: '{}'. Use 'd' for days, 'm' for months, 'y' for years.",
            unit_part
        )),
    }
}

fn handle_cache_command(args: CacheArgs) -> Result<()> {
    match args.command {
        CacheCommands::Prune { duration } => {
            println!("Pruning cache items older than {duration}...");
            let duration = parse_duration(&duration)?;
            let cutoff = Utc::now() - duration;

            let rt = Runtime::new().unwrap();
            let handle = rt.handle();
            handle.block_on(async {
                match enigmatick::models::cache::prune_cache_items(None, cutoff).await {
                    Ok(count) => println!("Successfully pruned {count} cache items."),
                    Err(e) => eprintln!("Error pruning cache: {e}"),
                }
            });
        }
        CacheCommands::Delete { url } => {
            println!("Attempting to delete cache item with URL: {url}...");
            let rt = Runtime::new().unwrap();
            let handle = rt.handle();
            handle.block_on(async {
                match enigmatick::models::cache::delete_cache_item_by_url(None, url.clone()).await {
                    Ok(_) => println!("Successfully deleted cache item for URL: {url}."),
                    Err(e) => eprintln!("Error deleting cache item for URL {url}: {e}"),
                }
            });
        }
    }
    Ok(())
}

fn format_relative_time(datetime: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration_since = now.signed_duration_since(datetime);

    // Handle cases where datetime is in the future (should ideally not happen for 'last_message_at')
    if duration_since < Duration::zero() {
        return "In the future".to_string(); // Or datetime.to_rfc3339() as a fallback
    }

    let days_since = duration_since.num_days();

    if days_since == 0 {
        return "Today".to_string();
    }
    if days_since == 1 {
        return "Yesterday".to_string();
    }
    if days_since < 7 {
        return format!("{days_since} days ago");
    }
    if days_since < 14 {
        return "Last week".to_string();
    }
    if days_since < (4 * 7) {
        // Up to 3 full weeks
        return format!("{} weeks ago", duration_since.num_weeks());
    }

    // Approximate months. Using 30 days as a rough guide for a month.
    // More precise would be (days_since as f64 / 30.4375).round() as i64 for months_ago
    let months_since_approx = (days_since as f64 / 30.4375).round() as i64;

    if months_since_approx == 1 {
        return "Last month".to_string();
    }
    if months_since_approx < 12 {
        return format!("{months_since_approx} months ago");
    }

    // Approximate years
    let years_since_approx = (days_since as f64 / 365.2425).round() as i64;

    if years_since_approx == 1 {
        return "Last year".to_string();
    }
    if years_since_approx > 1 {
        return format!("{years_since_approx} years ago");
    }

    // Fallback for very recent but not caught (e.g. just over 3 weeks but not quite "Last month" by rounding)
    // or if somehow years_since_approx is 0 after months_since_approx >= 12 (unlikely with current logic)
    // This also covers the case where it's just under a year but more than 11 months by strict rounding.
    // The most common case here would be "X weeks ago" if it didn't hit "Last month".
    format!("{} weeks ago", duration_since.num_weeks())
}

fn print_instance_table(instances: Vec<Instance>) {
    if instances.is_empty() {
        println!("No instances found.");
        return;
    }

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL); // Use a modern UTF-8 preset
    table.set_header(vec![
        Cell::new("Domain Name").add_attribute(Attribute::Bold),
        Cell::new("Blocked").add_attribute(Attribute::Bold),
        Cell::new("Last Message At").add_attribute(Attribute::Bold),
    ]);
    table.set_constraints(vec![
        ColumnConstraint::LowerBoundary(Width::Fixed(40)), // For "Domain Name" column (index 0)
        ColumnConstraint::ContentWidth,                    // For "Blocked" column (index 1)
        ColumnConstraint::ContentWidth,                    // For "Last Message At" column (index 2)
    ]);

    for instance in instances {
        let blocked_status_text = if instance.blocked { "Yes" } else { "No" };
        let blocked_cell = if instance.blocked {
            Cell::new(blocked_status_text).fg(Color::Red)
        } else {
            Cell::new(blocked_status_text).fg(Color::Green)
        };

        table.add_row(vec![
            Cell::new(instance.domain_name),
            blocked_cell,
            Cell::new(format_relative_time(instance.last_message_at)),
        ]);
    }
    println!("{table}");
}

fn print_instance_detail(instance: Instance, operation_description: &str) {
    println!("Instance details ({operation_description}):");
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL); // Use a modern UTF-8 preset
    table.set_header(vec![
        Cell::new("Property").add_attribute(Attribute::Bold),
        Cell::new("Value").add_attribute(Attribute::Bold),
    ]);
    table.set_constraints(vec![
        ColumnConstraint::ContentWidth,                    // For "Property" column (index 0)
        ColumnConstraint::LowerBoundary(Width::Fixed(40)), // For "Value" column (index 1)
    ]);

    let blocked_status_text = if instance.blocked { "Yes" } else { "No" };
    let blocked_value_cell = if instance.blocked {
        Cell::new(blocked_status_text).fg(Color::Red)
    } else {
        Cell::new(blocked_status_text).fg(Color::Green)
    };

    table.add_row(vec![
        Cell::new("Domain Name").add_attribute(Attribute::Italic),
        Cell::new(&instance.domain_name),
    ]);
    table.add_row(vec![
        Cell::new("Blocked").add_attribute(Attribute::Italic),
        blocked_value_cell,
    ]);
    table.add_row(vec![
        Cell::new("Last Message At").add_attribute(Attribute::Italic),
        Cell::new(format_relative_time(instance.last_message_at)),
    ]);
    table.add_row(vec![
        Cell::new("Created At").add_attribute(Attribute::Italic),
        Cell::new(instance.created_at.to_rfc3339()),
    ]);
    table.add_row(vec![
        Cell::new("Updated At").add_attribute(Attribute::Italic),
        Cell::new(instance.updated_at.to_rfc3339()),
    ]);
    println!("{table}");
}

const DEFAULT_CLAP_PAGE_SIZE: i64 = 20; // Keep this for when page_size is None

fn handle_instance_command(args: InstanceArgs) -> Result<()> {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    match args.command {
        InstanceCommands::List {
            mut page,
            page_size,
            sort: sort_string_opt, // This is Option<String>
        } => {
            let lib_sort_vec: Option<Vec<LibSortParam>> = sort_string_opt
                .map_or(Ok(None), |s| { // If sort_string_opt is None, default to Ok(None)
                    if s.is_empty() {
                        Ok(None) // If the string is empty, treat as no sort params
                    } else {
                        parse_sort_string_to_lib_params(&s)
                            .map(Some) // If parsing is Ok(params), map to Some(params)
                            .map_err(|e| { // If parsing is Err(parse_err_str), convert to anyhow::Error
                                eprintln!("Error parsing --sort argument: {e}");
                                anyhow::anyhow!("Invalid --sort value: {e}")
                            })
                    }
                })?;

            // page_size is now Option<i64>
            let _raw_mode_guard = match RawModeGuard::new() {
                Ok(guard) => guard,
                Err(e) => {
                    eprintln!(
                        "Failed to enable raw mode: {e}. Displaying first page non-interactively."
                    );
                    // If raw mode fails, use user-provided page_size or the default.
                    let non_raw_page_size = page_size.unwrap_or(DEFAULT_CLAP_PAGE_SIZE);
                    handle.block_on(async {
                        match instance_model::get_all_instances_paginated(
                            None,
                            1,
                            non_raw_page_size.max(1),
                            lib_sort_vec.clone(), // Pass the converted lib_sort_vec
                        )
                        .await
                        {
                            Ok(instances) => {
                                print_instance_table(instances);
                            }
                            Err(db_err) => eprintln!("Error listing instances: {db_err}"),
                        }
                    });
                    return Ok(()); // Exit early if raw mode fails
                }
            };

            // Initial clear and message (optional, as loop also clears)
            // execute!(stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            // println!("Listing instances...");

            let result: Result<()> = handle.block_on(async {
                let mut effective_page_size: i64;

                if let Some(user_specified_page_size) = page_size {
                    // User explicitly provided --page-size
                    effective_page_size = user_specified_page_size;
                } else {
                    // User did NOT provide --page-size, use default and try dynamic calculation
                    effective_page_size = DEFAULT_CLAP_PAGE_SIZE; // Start with the default

                    if let Ok((_cols, rows)) = terminal::size() {
                        // Overhead breakdown:
                        // - Table Top Border: 1 line
                        // - Table Header Text: 1 line
                        // - Table Header Separator: 1 line
                        // - Empty line before prompt: 1 line
                        // - Prompt line: 1 line
                        // Total lines NOT part of the repeating data-item blocks = 5 lines.
                        const TOTAL_FIXED_LINES_OVERHEAD: i64 = 5;
                        // Each data item (text + its separator line) takes 2 lines.
                        const LINES_PER_DATA_ITEM_BLOCK: i64 = 2;

                        // Minimum lines needed to display 1 data item and all overhead:
                        // TOTAL_FIXED_LINES_OVERHEAD + 1 * LINES_PER_DATA_ITEM_BLOCK = 5 + 2 = 7 lines.
                        if (rows as i64) >= TOTAL_FIXED_LINES_OVERHEAD + LINES_PER_DATA_ITEM_BLOCK {
                            let available_lines_for_data_item_blocks =
                                (rows as i64) - TOTAL_FIXED_LINES_OVERHEAD;
                            let calculated_data_items =
                                available_lines_for_data_item_blocks / LINES_PER_DATA_ITEM_BLOCK;
                            effective_page_size = calculated_data_items;
                        } else {
                            effective_page_size = 1;
                        }
                    }
                }
                // If terminal::size() fails, effective_page_size remains DEFAULT_CLAP_PAGE_SIZE.

                // Ensure page size is at least 1.
                effective_page_size = effective_page_size.max(1);

                loop {
                    // Clear screen and move cursor to top-left for each new page
                    execute!(stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))
                        .map_err(|e| anyhow::anyhow!("Terminal clear failed: {}", e))?;

                    // Use effective_page_size here
                    match instance_model::get_all_instances_paginated(
                        None,
                        page,
                        effective_page_size,
                        lib_sort_vec.clone(), // Pass the converted lib_sort_vec
                    )
                    .await
                    {
                        Ok(instances_data) => {
                            // Renamed to avoid conflict
                            if instances_data.is_empty() {
                                let message = if page == 1 {
                                    "No instances found."
                                } else {
                                    "No more instances."
                                };
                                execute!(
                                    stdout(),
                                    crossterm::style::Print(message),
                                    crossterm::style::Print("\r\n") // Explicit CR+LF
                                )
                                .map_err(|e| anyhow::anyhow!("Print failed: {}", e))?;
                                execute!(
                                    stdout(),
                                    crossterm::style::Print("Press any key to exit."),
                                    crossterm::style::Print("\r\n") // Explicit CR+LF
                                )
                                .map_err(|e| anyhow::anyhow!("Print failed: {}", e))?;

                                // Wait for any key press to exit
                                loop {
                                    if event::poll(std::time::Duration::from_millis(100))
                                        .map_err(|e| anyhow::anyhow!("Event poll failed: {}", e))?
                                    {
                                        if let Event::Key(_) = event::read().map_err(|e| {
                                            anyhow::anyhow!("Event read failed: {}", e)
                                        })? {
                                            break;
                                        }
                                    }
                                }
                                return Ok(()); // Exit async block
                            }

                            // Manually construct and print table content using crossterm
                            let mut table_display = Table::new();
                            table_display.load_preset(presets::UTF8_FULL); // Use a modern UTF-8 preset
                            table_display.set_header(vec![
                                Cell::new("Domain Name").add_attribute(Attribute::Bold),
                                Cell::new("Blocked").add_attribute(Attribute::Bold),
                                Cell::new("Last Message At").add_attribute(Attribute::Bold),
                            ]);
                            table_display.set_constraints(vec![
                                ColumnConstraint::LowerBoundary(Width::Fixed(40)), // For "Domain Name" column (index 0)
                                ColumnConstraint::ContentWidth,                    // For "Blocked" column (index 1)
                                ColumnConstraint::ContentWidth,                    // For "Last Message At" column (index 2)
                            ]);

                            for instance_item in instances_data {
                                let blocked_status_text =
                                    if instance_item.blocked { "Yes" } else { "No" };
                                let blocked_cell = if instance_item.blocked {
                                    Cell::new(blocked_status_text).fg(Color::Red)
                                } else {
                                    Cell::new(blocked_status_text).fg(Color::Green)
                                };

                                table_display.add_row(vec![
                                    Cell::new(instance_item.domain_name),
                                    blocked_cell,
                                    Cell::new(format_relative_time(instance_item.last_message_at)),
                                ]);
                            }
                            for line in table_display.to_string().lines() {
                                execute!(
                                    stdout(),
                                    crossterm::style::Print(line),
                                    crossterm::style::Print("\r\n") // Explicit CR+LF
                                )
                                .map_err(|e| anyhow::anyhow!("Table print failed: {}", e))?;
                            }

                            execute!(stdout(), crossterm::style::Print("\r\n"))
                                .map_err(|e| anyhow::anyhow!("Print failed: {}", e))?;
                            let prompt_line =
                                format!("Page {page}. Press Space for next, Q or Esc to quit. ");
                            execute!(
                                stdout(),
                                crossterm::style::Print(&prompt_line) // Print the prompt without a trailing newline.
                            )
                            .map_err(|e| anyhow::anyhow!("Prompt print failed: {}", e))?;
                            // The cursor will now be at the end of prompt_line.

                            // Input loop for current page
                            loop {
                                if event::poll(std::time::Duration::from_millis(100))
                                    .map_err(|e| anyhow::anyhow!("Event poll failed: {}", e))?
                                {
                                    if let Event::Key(KeyEvent {
                                        code, modifiers, ..
                                    }) = event::read()
                                        .map_err(|e| anyhow::anyhow!("Event read failed: {}", e))?
                                    {
                                        // Handle Ctrl+C for graceful exit
                                        if modifiers == KeyModifiers::CONTROL
                                            && code == KeyCode::Char('c')
                                        {
                                            execute!(stdout(), cursor::Show).ok();
                                            disable_raw_mode().ok();
                                            execute!(
                                                // Use crossterm for exit message
                                                stdout(),
                                                crossterm::style::Print(
                                                    "\rCtrl+C pressed. Exiting..."
                                                ),
                                                crossterm::style::Print("\r\n")
                                            )
                                            .ok();
                                            return Err(anyhow::anyhow!(
                                                "Operation cancelled by user (Ctrl+C)"
                                            ));
                                        }

                                        match code {
                                            KeyCode::Char(' ') => {
                                                page += 1;
                                                break; // Break from input loop, fetch next page
                                            }
                                            KeyCode::Char('q')
                                            | KeyCode::Char('Q')
                                            | KeyCode::Esc => {
                                                execute!(
                                                    stdout(),
                                                    cursor::MoveToColumn(0), // Move to beginning of current line
                                                    Clear(ClearType::CurrentLine), // Clear the current line
                                                    crossterm::style::Print("\rExiting list..."), // \r to ensure start of line
                                                    crossterm::style::Print("\r\n") // Proper newline
                                                )
                                                .map_err(|e| {
                                                    anyhow::anyhow!("Print failed: {}", e)
                                                })?;
                                                return Ok(());
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                // If poll times out, loop continues, effectively waiting for input.
                            }
                        }
                        Err(e) => {
                            // Ensure cursor is visible and terminal is somewhat reset before printing error
                            execute!(stdout(), cursor::Show).map_err(|ce| {
                                anyhow::anyhow!(
                                    "Failed to show cursor: {}. Original error: {}",
                                    ce,
                                    e
                                )
                            })?;
                            let error_msg = format!("\rError listing instances: {e}");
                            execute!(
                                // Use crossterm for error message
                                stdout(),
                                crossterm::style::Print(&error_msg),
                                crossterm::style::Print("\r\n")
                            )
                            .map_err(|ce| {
                                anyhow::anyhow!("Failed to print error: {ce}. Original error: {e}")
                            })?;
                            return Err(e); // Propagate the original error
                        }
                    }
                }
            }); // Closes the async block

            // RawModeGuard will disable raw mode when it goes out of scope.
            // Ensure cursor is visible before exiting, as raw mode might hide it.
            if let Err(e) = execute!(stdout(), cursor::Show) {
                eprintln!("Warning: Failed to ensure cursor is visible: {e}");
            }

            result?
        }
        InstanceCommands::Block { domain_name } => {
            println!("Attempting to block instance: {domain_name}...");
            handle.block_on(async {
                match instance_model::get_instance_by_domain_name(None, domain_name.clone()).await {
                    Ok(Some(instance)) if instance.blocked => {
                        println!("Instance {domain_name} is already blocked.");
                        print_instance_detail(instance, "already blocked");
                    }
                    Ok(Some(_)) | Ok(None) => {
                        // Not blocked or does not exist, proceed to set block status
                        match instance_model::set_block_status(None, domain_name.clone(), true)
                            .await
                        {
                            Ok(instance) => {
                                print_instance_detail(instance, "blocked successfully");
                            }
                            Err(e) => eprintln!("Error blocking instance {domain_name}: {e}"),
                        }
                    }
                    Err(e) => eprintln!("Error checking instance {domain_name}: {e}"),
                }
            });
            anyhow::Ok(())? // Add this line
        }
        InstanceCommands::Unblock { domain_name } => {
            println!("Attempting to unblock instance: {domain_name}...");
            handle.block_on(async {
                match instance_model::get_instance_by_domain_name(None, domain_name.clone()).await {
                    Ok(Some(instance)) if !instance.blocked => {
                        println!("Instance {domain_name} is already unblocked.");
                        print_instance_detail(instance, "already unblocked");
                    }
                    Ok(Some(_instance)) => {
                        // Exists and is blocked, proceed to unblock
                        match instance_model::set_block_status(None, domain_name.clone(), false)
                            .await
                        {
                            Ok(instance) => {
                                print_instance_detail(instance, "unblocked successfully");
                            }
                            Err(e) => eprintln!("Error unblocking instance {domain_name}: {e}"),
                        }
                    }
                    Ok(None) => {
                        eprintln!("Instance {domain_name} not found. Cannot unblock.");
                    }
                    Err(e) => eprintln!("Error checking instance {domain_name}: {e}"),
                }
            });
            anyhow::Ok(())? // Add this line
        }
        // ADD THIS NEW ARM FOR 'Get'
        InstanceCommands::Get { domain_name } => {
            println!("Attempting to retrieve instance: {domain_name}...");
            handle.block_on(async {
                match instance_model::get_instance_by_domain_name(None, domain_name.clone()).await {
                    Ok(Some(instance)) => {
                        print_instance_detail(instance, "retrieved");
                    }
                    Ok(None) => {
                        eprintln!("Instance {domain_name} not found.");
                    }
                    Err(e) => {
                        eprintln!("Error retrieving instance {domain_name}: {e}");
                    }
                }
            });
            anyhow::Ok(())?
        }
    }
    Ok(())
}

fn handle_system_user() -> Result<()> {
    let system_user = (*SYSTEM_USER).clone();

    println!("setup system user: {system_user}");
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();
    handle.block_on(async {
        if create_user(
            None,
            NewUser {
                username: system_user.clone(),
                password: Alphanumeric.sample_string(&mut rand::thread_rng(), 16),
                display_name: "System User".to_string(),
                client_public_key: None,
                client_private_key: None,
                olm_pickled_account: None,
                olm_pickled_account_hash: None,
                olm_identity_key: None,
                salt: None,
                kind: Some(ActorType::Application),
            },
        )
        .await
        .is_ok()
        {
            println!("system user created.");
        } else {
            println!("failed to create system user.");
        }
    });

    Ok(())
}
