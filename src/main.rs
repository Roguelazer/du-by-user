use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;

use itertools::Itertools;

const DIVISORS_SI: [(u64, &'static str); 4] = [
    (1_000_000_000_000, "T"),
    (1_000_000_000, "G"),
    (1_000_000, "M"),
    (1_000, "K"),
];
const DIVISORS_NON_SI: [(u64, &'static str); 4] = [
    (1_099_511_627_776, "T"),
    (1_073_741_824, "G"),
    (1_048_576, "M"),
    (1_024, "K"),
];

fn cli() -> clap::Command<'static> {
    clap::Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author("James Brown <jbrown@easypost.com>")
        .arg(
            clap::Arg::new("path")
                .default_value(".")
                .help("Path to scan"),
        )
        .arg(
            clap::Arg::new("bytes")
                .short('b')
                .long("bytes")
                .takes_value(false)
                .help("Output number of bytes"),
        )
        .arg(
            clap::Arg::new("kilobytes")
                .short('k')
                .long("kilobytes")
                .takes_value(false)
                .help("Output kilobytes"),
        )
        .arg(
            clap::Arg::new("megabytes")
                .short('m')
                .long("megabytes")
                .takes_value(false)
                .help("Output megabytes"),
        )
        .arg(
            clap::Arg::new("gigabytes")
                .short('g')
                .long("gigabytes")
                .takes_value(false)
                .help("Output gigabytes"),
        )
        .arg(
            clap::Arg::new("human")
                .short('h')
                .long("human")
                .takes_value(false)
                .help("Output human-readable sizes, whatever that means"),
        )
        .arg(
            clap::Arg::new("si")
                .long("si")
                .takes_value(false)
                .help("Interpret things as powers of 10 instead of powers of 2"),
        )
        .group(clap::ArgGroup::new("output").args(&[
            "bytes",
            "kilobytes",
            "megabytes",
            "gigabytes",
            "human",
        ]))
}

#[derive(Debug)]
enum SizeMode {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
    Human,
}

impl SizeMode {
    fn from_matches(matches: &clap::ArgMatches) -> Self {
        if matches.is_present("gigabytes") {
            Self::Gigabytes
        } else if matches.is_present("megabytes") {
            Self::Megabytes
        } else if matches.is_present("kilobytes") {
            Self::Kilobytes
        } else if matches.is_present("human") {
            Self::Human
        } else {
            Self::Bytes
        }
    }
}

struct FormattedSize<'s> {
    size: u64,
    formatter: &'s SizeFormatter,
}

impl<'s> std::fmt::Display for FormattedSize<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let (base, unit) = self.formatter.get_parts(self.size);
        write!(f, "{}", base)?;
        if let Some(unit) = unit {
            write!(f, "{}", unit)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct SizeFormatter {
    mode: SizeMode,
    si: bool,
}

impl SizeFormatter {
    fn from_matches(matches: &clap::ArgMatches) -> Self {
        Self {
            si: matches.is_present("si"),
            mode: SizeMode::from_matches(matches),
        }
    }

    fn get_parts_divisor(&self, size: u64, divisor: u64) -> (u64, Option<&'static str>) {
        (size / divisor, None)
    }

    fn get_parts_human(&self, size: u64) -> (u64, Option<&'static str>) {
        let divisors = if self.si {
            DIVISORS_SI
        } else {
            DIVISORS_NON_SI
        };
        for (divisor, unit) in divisors {
            if size > divisor * 10 {
                return (size / divisor, Some(unit));
            }
        }
        return (size, Some("B"));
    }

    fn get_parts(&self, size: u64) -> (u64, Option<&'static str>) {
        match (&self.mode, self.si) {
            (SizeMode::Bytes, _) => (size, None),
            (SizeMode::Kilobytes, false) => self.get_parts_divisor(size, 1024),
            (SizeMode::Kilobytes, true) => self.get_parts_divisor(size, 1000),
            (SizeMode::Megabytes, false) => self.get_parts_divisor(size, 1048576),
            (SizeMode::Megabytes, true) => self.get_parts_divisor(size, 1000000),
            (SizeMode::Gigabytes, false) => self.get_parts_divisor(size, 1073741824),
            (SizeMode::Gigabytes, true) => self.get_parts_divisor(size, 1000000000),
            (SizeMode::Human, _) => self.get_parts_human(size),
        }
    }

    fn wrap(&self, size: u64) -> FormattedSize {
        FormattedSize {
            size,
            formatter: &self,
        }
    }
}

fn main() {
    let matches = cli().get_matches();
    let formatter = SizeFormatter::from_matches(&matches);
    let mut by_user: HashMap<u32, u64> = HashMap::new();
    let walker = walkdir::WalkDir::new(matches.value_of_t_or_exit::<std::path::PathBuf>("path"))
        .follow_links(false);
    for entry in walker {
        if let Ok(metadata) = entry.and_then(|e| e.metadata()) {
            if metadata.is_file() {
                *by_user.entry(metadata.uid()).or_insert_with(|| 0) += metadata.size();
            }
        }
    }
    by_user
        .into_iter()
        .sorted_by_key(|&(_, v)| v)
        .for_each(|(user_id, size)| {
            let u = users::get_user_by_uid(user_id)
                .map(|u| u.name().to_string_lossy().into_owned())
                .unwrap_or_else(|| user_id.to_string());
            println!("{}\t{}", formatter.wrap(size), u);
        })
}

#[cfg(test)]
mod tests {
    use super::cli;

    #[test]
    fn test_debug_assert_cli() {
        cli().debug_assert()
    }
}
