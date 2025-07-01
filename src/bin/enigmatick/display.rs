use chrono::{DateTime, Utc};
use comfy_table::{presets, Attribute, Cell, Color, ColumnConstraint, Table, Width};
use enigmatick::models::instances::Instance;

pub fn format_relative_time(datetime: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration_since = now.signed_duration_since(datetime);

    // Handle cases where datetime is in the future (should ideally not happen for 'last_message_at')
    if duration_since < chrono::Duration::zero() {
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

pub fn print_instance_table(instances: Vec<Instance>) {
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

pub fn print_instance_detail(instance: Instance, operation_description: &str) {
    println!("Instance details ({operation_description}):");
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL); // Use a modern UTF-8 preset
    table.set_header(vec![
        Cell::new("Property").add_attribute(Attribute::Bold),
        Cell::new("Value").add_attribute(Attribute::Bold),
    ]);
    table.set_constraints(vec![
        ColumnConstraint::ContentWidth, // For "Property" column (index 0)
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
