use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{presets, Attribute, Cell, Color, ColumnConstraint, Table, Width};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use enigmatick::models::instances::{
    self as instance_model_ops, SortDirection as LibSortDirection, SortField as LibSortField,
    SortParam as LibSortParam,
};
use enigmatick::models::{
    activities::delete_activities_by_domain_pattern, actors::delete_actors_by_domain_pattern,
    cache::delete_cache_items_by_server_pattern, follows::delete_follows_by_domain_pattern,
    objects::delete_objects_by_domain_pattern,
};
use std::io::stdout;
use tokio::runtime::Runtime;

use crate::display::{format_relative_time, print_instance_detail, print_instance_table};

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
            eprintln!(
                "Error: Failed to disable raw mode: {e}. Terminal might be in an unexpected state."
            );
        }
    }
}

#[derive(Parser)]
pub struct InstanceArgs {
    #[command(subcommand)]
    pub command: InstanceCommands,
}

#[derive(Subcommand)]
pub enum InstanceCommands {
    /// List instances with pagination
    List {
        #[clap(long, default_value = "1")]
        page: i64,
        #[clap(long)]
        page_size: Option<i64>,
        /// Sort order: "field[:direction][,field[:direction]...]"
        /// Fields: domain, blocked, last. Directions: asc, desc
        #[clap(long)]
        sort: Option<String>,
    },
    /// Block instance by domain name
    Block { domain_name: String },
    /// Unblock instance by domain name
    Unblock { domain_name: String },
    /// Get instance details by domain name
    Get { domain_name: String },
}

// Helper function to parse a single sort field string like "blocked" or "blocked:asc"
fn parse_one_lib_sort_param(s: &str) -> Result<LibSortParam, String> {
    let parts: Vec<&str> = s.split(':').collect();

    let field_str = parts[0];
    let direction_str_opt = parts.get(1).copied();

    if parts.len() > 2 {
        return Err(format!(
            "Invalid sort format: '{s}'. Expected 'field' or 'field:direction'. Too many colons."
        ));
    }

    let field = match field_str.to_lowercase().as_str() {
        "domain" | "domain_name" | "name" => LibSortField::DomainName,
        "blocked" => LibSortField::Blocked,
        "last" | "last_message_at" | "lastmessageat" => LibSortField::LastMessageAt,
        _ => return Err(format!("Unknown sort field: '{field_str}'")),
    };

    let direction = match direction_str_opt {
        Some(dir_str) => match dir_str.to_lowercase().as_str() {
            "asc" | "ascending" => LibSortDirection::Asc,
            "desc" | "descending" => LibSortDirection::Desc,
            _ => return Err(format!("Unknown sort direction: '{dir_str}'")),
        },
        None => LibSortDirection::Asc,
    };

    Ok(LibSortParam { field, direction })
}

fn parse_sort_string_to_lib_params(s: &str) -> Result<Vec<LibSortParam>, String> {
    s.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(parse_one_lib_sort_param)
        .collect()
}

const DEFAULT_CLAP_PAGE_SIZE: i64 = 20;

pub fn handle_instance_command(args: InstanceArgs) -> Result<()> {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    match args.command {
        InstanceCommands::List {
            mut page,
            page_size,
            sort: sort_string_opt,
        } => {
            let lib_sort_vec: Option<Vec<LibSortParam>> =
                sort_string_opt.map_or(Ok(None), |s| {
                    if s.is_empty() {
                        Ok(None)
                    } else {
                        parse_sort_string_to_lib_params(&s).map(Some).map_err(|e| {
                            eprintln!("Error parsing --sort argument: {e}");
                            anyhow::anyhow!("Invalid --sort value: {e}")
                        })
                    }
                })?;

            let _raw_mode_guard = match RawModeGuard::new() {
                Ok(guard) => guard,
                Err(e) => {
                    eprintln!(
                        "Failed to enable raw mode: {e}. Displaying first page non-interactively."
                    );
                    let non_raw_page_size = page_size.unwrap_or(DEFAULT_CLAP_PAGE_SIZE);
                    handle.block_on(async {
                        match instance_model_ops::get_all_instances_paginated(
                            None,
                            1,
                            non_raw_page_size.max(1),
                            lib_sort_vec.clone(),
                        )
                        .await
                        {
                            Ok(instances) => {
                                print_instance_table(instances);
                            }
                            Err(db_err) => eprintln!("Error listing instances: {db_err}"),
                        }
                    });
                    return Ok(());
                }
            };

            let result: Result<()> = handle.block_on(async {
                let mut effective_page_size: i64;

                if let Some(user_specified_page_size) = page_size {
                    effective_page_size = user_specified_page_size;
                } else {
                    effective_page_size = DEFAULT_CLAP_PAGE_SIZE;

                    if let Ok((_cols, rows)) = terminal::size() {
                        const TOTAL_FIXED_LINES_OVERHEAD: i64 = 5;
                        const LINES_PER_DATA_ITEM_BLOCK: i64 = 2;

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

                effective_page_size = effective_page_size.max(1);

                loop {
                    execute!(stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))
                        .map_err(|e| anyhow::anyhow!("Terminal clear failed: {}", e))?;

                    match instance_model_ops::get_all_instances_paginated(
                        None,
                        page,
                        effective_page_size,
                        lib_sort_vec.clone(),
                    )
                    .await
                    {
                        Ok(instances_data) => {
                            if instances_data.is_empty() {
                                let message = if page == 1 {
                                    "No instances found."
                                } else {
                                    "No more instances."
                                };
                                execute!(
                                    stdout(),
                                    crossterm::style::Print(message),
                                    crossterm::style::Print("\r\n")
                                )
                                .map_err(|e| anyhow::anyhow!("Print failed: {}", e))?;
                                execute!(
                                    stdout(),
                                    crossterm::style::Print("Press any key to exit."),
                                    crossterm::style::Print("\r\n")
                                )
                                .map_err(|e| anyhow::anyhow!("Print failed: {}", e))?;

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
                                return Ok(());
                            }

                            let mut table_display = Table::new();
                            table_display.load_preset(presets::UTF8_FULL);
                            table_display.set_header(vec![
                                Cell::new("Domain Name").add_attribute(Attribute::Bold),
                                Cell::new("Blocked").add_attribute(Attribute::Bold),
                                Cell::new("Last Message At").add_attribute(Attribute::Bold),
                            ]);
                            table_display.set_constraints(vec![
                                ColumnConstraint::LowerBoundary(Width::Fixed(40)),
                                ColumnConstraint::ContentWidth,
                                ColumnConstraint::ContentWidth,
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
                                    crossterm::style::Print("\r\n")
                                )
                                .map_err(|e| anyhow::anyhow!("Table print failed: {}", e))?;
                            }

                            execute!(stdout(), crossterm::style::Print("\r\n"))
                                .map_err(|e| anyhow::anyhow!("Print failed: {}", e))?;
                            let prompt_line =
                                format!("Page {page}. Press Space for next, Q or Esc to quit. ");
                            execute!(stdout(), crossterm::style::Print(&prompt_line))
                                .map_err(|e| anyhow::anyhow!("Prompt print failed: {}", e))?;

                            loop {
                                if event::poll(std::time::Duration::from_millis(100))
                                    .map_err(|e| anyhow::anyhow!("Event poll failed: {}", e))?
                                {
                                    if let Event::Key(KeyEvent {
                                        code, modifiers, ..
                                    }) = event::read()
                                        .map_err(|e| anyhow::anyhow!("Event read failed: {}", e))?
                                    {
                                        if modifiers == KeyModifiers::CONTROL
                                            && code == KeyCode::Char('c')
                                        {
                                            execute!(stdout(), cursor::Show).ok();
                                            disable_raw_mode().ok();
                                            execute!(
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
                                                break;
                                            }
                                            KeyCode::Char('q')
                                            | KeyCode::Char('Q')
                                            | KeyCode::Esc => {
                                                execute!(
                                                    stdout(),
                                                    cursor::MoveToColumn(0),
                                                    Clear(ClearType::CurrentLine),
                                                    crossterm::style::Print("\rExiting list..."),
                                                    crossterm::style::Print("\r\n")
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
                            }
                        }
                        Err(e) => {
                            execute!(stdout(), cursor::Show).map_err(|ce| {
                                anyhow::anyhow!(
                                    "Failed to show cursor: {}. Original error: {}",
                                    ce,
                                    e
                                )
                            })?;
                            let error_msg = format!("\rError listing instances: {e}");
                            execute!(
                                stdout(),
                                crossterm::style::Print(&error_msg),
                                crossterm::style::Print("\r\n")
                            )
                            .map_err(|ce| {
                                anyhow::anyhow!("Failed to print error: {ce}. Original error: {e}")
                            })?;
                            return Err(e);
                        }
                    }
                }
            });

            if let Err(e) = execute!(stdout(), cursor::Show) {
                eprintln!("Warning: Failed to ensure cursor is visible: {e}");
            }

            result?
        }
        InstanceCommands::Block { domain_name } => {
            println!("Attempting to block instance: {domain_name}...");
            handle.block_on(async {
                let instance_result =
                    instance_model_ops::get_instance_by_domain_name(None, domain_name.clone())
                        .await;

                match instance_result {
                    Ok(Some(instance)) if instance.blocked => {
                        println!("Instance {domain_name} is already blocked.");
                        print_instance_detail(instance, "already blocked");
                    }
                    Ok(Some(_)) | Ok(None) => {
                        match instance_model_ops::set_block_status(None, domain_name.clone(), true)
                            .await
                        {
                            Ok(instance) => {
                                println!(
                                    "Instance blocked successfully. Cleaning up associated data..."
                                );

                                async fn cleanup_domain_data(domain: &str) {
                                    match delete_objects_by_domain_pattern(None, domain.to_string())
                                        .await
                                    {
                                        Ok(count) => {
                                            println!("Deleted {count} objects from blocked domain.")
                                        }
                                        Err(e) => eprintln!("Error deleting objects: {e}"),
                                    }

                                    match delete_activities_by_domain_pattern(
                                        None,
                                        domain.to_string(),
                                    )
                                    .await
                                    {
                                        Ok(count) => println!(
                                            "Deleted {count} activities from blocked domain."
                                        ),
                                        Err(e) => eprintln!("Error deleting activities: {e}"),
                                    }

                                    match delete_follows_by_domain_pattern(None, domain.to_string())
                                        .await
                                    {
                                        Ok(count) => println!(
                                            "Deleted {count} followers from blocked domain."
                                        ),
                                        Err(e) => eprintln!("Error deleting followers: {e}"),
                                    }

                                    match delete_actors_by_domain_pattern(None, domain.to_string())
                                        .await
                                    {
                                        Ok(count) => {
                                            println!("Deleted {count} actors from blocked domain.")
                                        }
                                        Err(e) => eprintln!("Error deleting actors: {e}"),
                                    }

                                    match delete_cache_items_by_server_pattern(
                                        None,
                                        domain.to_string(),
                                    )
                                    .await
                                    {
                                        Ok(deleted_items) => {
                                            let message = if deleted_items.is_empty() {
                                                "No cache items found for blocked domain."
                                                    .to_string()
                                            } else {
                                                format!(
                                                    "Deleted {} cache items from blocked domain.",
                                                    deleted_items.len()
                                                )
                                            };
                                            println!("{message}");
                                        }
                                        Err(e) => eprintln!("Error deleting cache items: {e}"),
                                    }
                                }

                                cleanup_domain_data(&domain_name).await;
                                print_instance_detail(
                                    instance,
                                    "blocked successfully with cleanup completed",
                                );
                            }
                            Err(e) => eprintln!("Error blocking instance {domain_name}: {e}"),
                        }
                    }
                    Err(e) => eprintln!("Error checking instance {domain_name}: {e}"),
                }
            });
            anyhow::Ok(())?
        }
        InstanceCommands::Unblock { domain_name } => {
            println!("Attempting to unblock instance: {domain_name}...");
            handle.block_on(async {
                let instance_result =
                    instance_model_ops::get_instance_by_domain_name(None, domain_name.clone())
                        .await;

                match instance_result {
                    Ok(Some(instance)) if !instance.blocked => {
                        println!("Instance {domain_name} is already unblocked.");
                        print_instance_detail(instance, "already unblocked");
                    }
                    Ok(Some(_instance)) => {
                        let unblock_result =
                            instance_model_ops::set_block_status(None, domain_name.clone(), false)
                                .await;

                        match unblock_result {
                            Ok(instance) => {
                                print_instance_detail(instance, "unblocked successfully")
                            }
                            Err(e) => eprintln!("Error unblocking instance {domain_name}: {e}"),
                        }
                    }
                    Ok(None) => eprintln!("Instance {domain_name} not found. Cannot unblock."),
                    Err(e) => eprintln!("Error checking instance {domain_name}: {e}"),
                }
            });
            anyhow::Ok(())?
        }
        InstanceCommands::Get { domain_name } => {
            println!("Attempting to retrieve instance: {domain_name}...");
            handle.block_on(async {
                let instance_result =
                    instance_model_ops::get_instance_by_domain_name(None, domain_name.clone())
                        .await;

                match instance_result {
                    Ok(Some(instance)) => print_instance_detail(instance, "retrieved"),
                    Ok(None) => eprintln!("Instance {domain_name} not found."),
                    Err(e) => eprintln!("Error retrieving instance {domain_name}: {e}"),
                }
            });
            anyhow::Ok(())?
        }
    }
    Ok(())
}
