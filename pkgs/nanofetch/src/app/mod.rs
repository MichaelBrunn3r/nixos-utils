#![allow(clippy::cast_precision_loss)]

use std::net::IpAddr;

use chrono::{Local, TimeDelta};
use clcry::{
    Buffer, Color, Constraints, Flexible, Grid, GridTrack, ProgressBar, Rect, Sized, Span, Style,
    View, hflex, hstack, vstack,
};
use terminal_size::{Width, terminal_size};

use crate::data_sources::{CpuInfo, Load, Uptime};
use crate::gather::Data;

const MAX_WIDTH: usize = 100;

pub fn render(data: &Data) -> String {
    let terminal = terminal_size();

    let mut sections: Vec<Box<dyn View>> = vec![facts(data)];
    let mut memory_rows = Vec::new();
    if let Some(memory) = &data.memory {
        memory_rows.push(memory_row("Mem", memory.used, memory.total));
    }
    if let Some(swap) = &data.swap {
        memory_rows.push(memory_row("Swap", swap.used, swap.total));
    }
    if let Some(load) = &data.load {
        let cpu_count = data.cpu.as_ref().map_or(1, |cpu| cpu.count).max(1);
        memory_rows.push(load_row(load, cpu_count));
    }
    if !memory_rows.is_empty() {
        sections.push(Box::new(
            Grid::new(memory_rows)
                .columns([GridTrack::Content, GridTrack::Flexible(1)])
                .column_gap(1),
        ));
    }
    if !data.net_ifaces.is_empty() {
        sections.push(net(data));
    }

    let mut screen = Sized::new(vstack![..sections].gap(1)).max_width(MAX_WIDTH);

    let width = terminal.map_or(usize::MAX, |(Width(width), _)| usize::from(width));
    let size = screen.measure(Constraints::at_most(width, usize::MAX));
    let width = terminal.map_or(size.width, |_| width);
    let height = size.height;
    let mut buffer = Buffer::new(width, height);
    screen.arrange(Rect::new(0, 0, width, height));
    screen.render(&mut buffer);
    if terminal.is_some() {
        buffer.to_ansi()
    } else {
        buffer.to_plain()
    }
}

fn net(data: &Data) -> Box<dyn View> {
    let mut rows = vec![vec![
        Box::new(Span::styled("Interface", Style::fg(Color::CYAN))) as Box<dyn View>,
        Box::new(Span::styled("MAC", Style::fg(Color::CYAN))) as Box<dyn View>,
        Box::new(Span::styled("IP", Style::fg(Color::CYAN))) as Box<dyn View>,
        Box::new(Span::styled("↓↑ GiB", Style::fg(Color::CYAN))) as Box<dyn View>,
    ]];
    for (name, iface) in &data.net_ifaces {
        let mac = iface.mac.as_deref().unwrap_or("");
        let ip = iface
            .ips
            .iter()
            .find(|ip| ip.parse::<IpAddr>().is_ok_and(|address| address.is_ipv4()))
            .or_else(|| iface.ips.first())
            .map_or("", String::as_str);
        let traffic = data
            .net_ifaces_stats
            .get(name)
            .map(|stats| format!("{} / {}", fmt_gib(stats.rx), fmt_gib(stats.tx)))
            .unwrap_or_default();
        rows.push(vec![
            Box::new(Span::new(name.clone())) as Box<dyn View>,
            Box::new(Span::new(mac.to_owned())) as Box<dyn View>,
            Box::new(Span::new(ip.to_owned())) as Box<dyn View>,
            Box::new(Span::new(traffic)) as Box<dyn View>,
        ]);
    }
    Box::new(
        vstack![
            Span::styled("Net", Style::fg(Color::GREEN)),
            Grid::new(rows)
                .columns([
                    GridTrack::Content,
                    GridTrack::Content,
                    GridTrack::Content,
                    GridTrack::Content,
                ])
                .column_gap(1),
        ]
        .gap(0),
    )
}

fn fmt_gib(bytes: u64) -> String {
    const GIB: u64 = 1 << 30;
    format!("{:.2}", bytes as f64 / GIB as f64)
}

fn facts(data: &Data) -> Box<dyn View> {
    let rows = stats_from(data).into_iter().map(|(label, value)| {
        Box::new(hstack![
            Span::styled(format!("{label} "), Style::fg(Color::GREEN)),
            Span::new(format!("{value} ")),
        ]) as Box<dyn View>
    });
    Box::new(hflex![..rows.collect()])
}

fn memory_row(label: &str, used: u64, total: u64) -> Vec<Box<dyn View>> {
    let progress = if total == 0 {
        0.0
    } else {
        used as f64 / total as f64
    };
    vec![
        Box::new(Span::styled(label, Style::fg(Color::GREEN))) as Box<dyn View>,
        Box::new(
            ProgressBar::new()
                .filled_style(Style::fg(Color::BLACK).bg(Color::WHITE))
                .empty_style(Style::fg(Color::WHITE).bg(Color::GRAY))
                .label(format!(
                    "{} ({:.0}%)",
                    fmt_memory(used, total),
                    progress * 100.0
                ))
                .progress(progress),
        ),
    ]
}

fn load_row(load: &Load, cpu_count: u32) -> Vec<Box<dyn View>> {
    let scale = f64::from(cpu_count);
    let bar = |label: &str, value: f64| {
        ProgressBar::new()
            .filled_style(Style::fg(Color::BLACK).bg(Color::WHITE))
            .empty_style(Style::fg(Color::WHITE).bg(Color::GRAY))
            .label(format!("{label} {value:.2}"))
            .progress(value / scale)
    };
    vec![
        Box::new(Span::styled("Load", Style::fg(Color::GREEN))) as Box<dyn View>,
        Box::new(
            Sized::new(
                hstack![
                    Flexible::new(bar("1m", load.one)),
                    Flexible::new(bar("5m", load.five)),
                    Flexible::new(bar("15m", load.fifteen)),
                ]
                .gap(1),
            )
            .max_width(MAX_WIDTH),
        ) as Box<dyn View>,
    ]
}

fn fmt_memory(used: u64, total: u64) -> String {
    const GIB: u64 = 1 << 30;
    if total >= GIB {
        let used = used as f64 / GIB as f64;
        let total = total as f64 / GIB as f64;
        if used < 1.0 {
            format!("{used:.2} / {total:.1} GiB")
        } else {
            format!("{used:.1} / {total:.1} GiB")
        }
    } else {
        format!("{} / {}", fmt_filesize(used), fmt_filesize(total))
    }
}

fn fmt_filesize(bytes: u64) -> String {
    const GIB: u64 = 1 << 30;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn stats_from(data: &Data) -> Vec<(String, String)> {
    let mut stats = Vec::new();
    if let Some(hostname) = &data.hostname {
        stats.push(("Host".into(), hostname.clone()));
    }
    if let Some(os) = &data.os {
        stats.push(("OS".into(), os.clone()));
    }
    if let Some(kernel) = &data.kernel {
        stats.push(("Kernel".into(), kernel.clone()));
    }
    if let Some(uptime) = &data.uptime {
        stats.push(("Uptime".into(), fmt_uptime(uptime)));
    }
    if let Some(cpu) = &data.cpu {
        stats.push(("CPU".into(), fmt_cpu(cpu)));
        stats.push(("Threads".into(), cpu.count.to_string()));
    }
    stats
}

fn fmt_uptime(uptime: &Uptime) -> String {
    let human = fmt_duration(uptime.duration);
    let boot = uptime
        .booted_at
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M");
    format!("{human} ({boot})")
}

fn fmt_cpu(cpu: &CpuInfo) -> String {
    format!("{} @ {:.2} GHz", cpu.brand, cpu.ghz)
}

fn fmt_duration(duration: TimeDelta) -> String {
    let total = u64::try_from(duration.num_seconds()).unwrap_or_default();
    let days = total / 86_400;
    let hours = (total % 86_400) / 3_600;
    let minutes = (total % 3_600) / 60;
    let seconds = total % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 {
        parts.push(format!("{seconds}s"));
    }
    if parts.is_empty() {
        "0s".to_owned()
    } else {
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use clcry::ContentAlignment;

    use super::*;
    use crate::data_sources::Memory;

    #[test]
    fn snapshots_memory_and_swap_rows() {
        let data = Data {
            memory: Some(Memory {
                used: 13 * (1 << 30) + 322_122_547,
                total: 31 * (1 << 30) + 107_374_182,
            }),
            swap: Some(Memory {
                used: 256 * (1 << 20),
                total: 16 * (1 << 30) + 966_367_642,
            }),
            ..Data::default()
        };

        let mut screen = hstack![Sized::new(
            Grid::new(vec![
                memory_row(
                    "Mem",
                    data.memory.as_ref().expect("memory is present").used,
                    data.memory.as_ref().expect("memory is present").total,
                ),
                memory_row(
                    "Swap",
                    data.swap.as_ref().expect("swap is present").used,
                    data.swap.as_ref().expect("swap is present").total,
                ),
            ])
            .columns([GridTrack::Content, GridTrack::Flexible(1),])
            .column_gap(1),
        )]
        .content_alignment(ContentAlignment::Center);
        let size = screen.measure(Constraints::at_most(80, usize::MAX));
        let mut buffer = Buffer::new(80, size.height);
        screen.arrange(Rect::new(0, 0, 80, size.height));
        screen.render(&mut buffer);

        insta::assert_snapshot!(buffer.to_plain());
    }
}
