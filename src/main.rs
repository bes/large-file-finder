use clap::{App, Arg};
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::borrow::{BorrowMut, Cow};
use std::cmp::max;
use std::error::Error;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

fn main() {
    let matches = App::new("Large file finder")
        .version("0.2.0")
        .arg(
            Arg::with_name("directory")
                .value_name("DIRECTORY")
                .help("The directory to scan for files and directories")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::with_name("percent")
                .env("PERCENT")
                .long("percent")
                .short("p")
                .takes_value(true)
                .default_value("50")
                .help("Show files and dirs larger than this percentage of the largest file")
                .long_help("Show all files and directories that are larger than X% of the largest found file."),
        )
        .arg(
            Arg::with_name("ignore")
                .env("IGNORE")
                .long("ignore")
                .short("i")
                .takes_value(true)
                .default_value("false")
                .help("Respect ignore files")
                .long_help("Respects ignore files when true, counts all files when false (default)."),
        )
        .get_matches();

    let path_str = match matches.value_of("directory") {
        None => {
            println!("Error: must provide a directory");
            return;
        }
        Some(path_str) => path_str,
    };

    let percent = match matches.value_of("percent") {
        Some(percent_str) => match f64::from_str(percent_str) {
            Ok(pct) => pct,
            Err(_) => panic!(),
        },
        None => panic!(),
    };

    let ignore = match matches.value_of("ignore") {
        Some(ignore) => bool::from_str(ignore).unwrap_or_else(|_| false),
        None => false,
    };

    let expanded_path_str = match shellexpand::full(path_str) {
        Ok(eps) => eps,
        Err(e) => {
            panic!("Error: {}", e);
        }
    };
    let mut base_dir = Dir::new(&expanded_path_str);

    match find_all_files_and_directories(&mut base_dir, ignore) {
        Ok(_) => (),
        Err(e) => {
            panic!("Error: {}", e);
        }
    }

    base_dir.calc_size();
    let total_size = base_dir.size();
    let largest_child = base_dir.largest_child();
    base_dir.print((largest_child as f64 * (percent / 100.0)) as u64);

    println!("Total size: {}", bytes_to_nice(total_size));
    println!("Largest child: {}", bytes_to_nice(largest_child));
}

fn find_all_files_and_directories(dir: &mut Dir, ignore: bool) -> Result<(), Box<dyn Error>> {
    // let path = Path::new(&dir.path);
    // let read_dir = read_dir(path)?;

    println!("Ignore {}", ignore);

    dir.children =
        // read_dir
        WalkBuilder::new(&dir.path).standard_filters(ignore).build()
        .map(|f| Arc::new(f))
        .par_bridge()
        .fold(
            || Arc::new(Mutex::new(Vec::<FsItem>::new())),
            |children, entry_result| {
                let entry = match entry_result.as_ref() {
                    Err(_) => return children,
                    Ok(de) => de,
                };

                let path = entry.path();
                let entry_path: &str = match path.to_str() {
                    None => panic!("oops"),
                    Some(t) => t,
                };

                let metadata = match entry.metadata() {
                    Ok(metadata) => metadata,
                    Err(e) => {
                        println!("{e}");
                        return children;
                    }
                };
                if !metadata.is_dir() {
                    let new_file = File::new(metadata.len(), entry_path);
                    children.lock().unwrap().push(FsItem::File(new_file));
                }
                children
            },
        )
        .reduce(
            || Arc::new(Mutex::new(Vec::<FsItem>::new())),
            |acc, val| {
                acc.lock().unwrap().append(val.lock().unwrap().borrow_mut());
                acc
            },
        );

    Ok(())
}

enum FsItem {
    File(File),
}

impl FsItem {
    fn as_file_data(&mut self) -> &mut dyn FileData {
        match self {
            FsItem::File(file) => file,
        }
    }
}

trait FileData {
    fn calc_size(&mut self);
    fn size(&self) -> u64;
    fn largest_child(&self) -> u64;
    fn path(&self) -> Cow<str>;
    fn is_file(&self) -> bool;
    fn print(&self, cutoff: u64);
}

struct Dir {
    path: String,
    children: Arc<Mutex<Vec<FsItem>>>,
    size: Option<u64>,
}

impl Dir {
    fn new(path: &str) -> Dir {
        Dir {
            path: path.to_owned(),
            children: Arc::new(Mutex::new(Vec::new())),
            size: None,
        }
    }
}

impl FileData for Dir {
    fn calc_size(&mut self) {
        let mut total_size: u64 = 0;
        for child in self.children.lock().unwrap().iter_mut() {
            let fd = child.as_file_data();
            fd.calc_size();
            total_size += fd.size();
        }
        self.size = Some(total_size);
    }

    fn size(&self) -> u64 {
        self.size.unwrap_or_else(|| 0)
    }

    fn largest_child(&self) -> u64 {
        self.children.lock().unwrap().iter_mut().fold(0, |v, f| {
            let fd = f.as_file_data();
            return max(v, fd.largest_child());
        })
    }

    fn path(&self) -> Cow<str> {
        return Cow::Borrowed(&self.path);
    }

    fn is_file(&self) -> bool {
        return false;
    }

    fn print(&self, cutoff: u64) {
        let sz = self.size();
        if sz >= cutoff {
            println!("{0: <8} d {1}", bytes_to_nice(sz), self.path);
            self.children
                .lock()
                .unwrap()
                .iter_mut()
                .map(|fsi| fsi.as_file_data())
                .for_each(|f| f.print(cutoff));
        }
    }
}

#[derive(Clone)]
struct File {
    size: u64,
    path: String,
}

impl File {
    fn new(size: u64, path: &str) -> File {
        File {
            size,
            path: path.to_owned(),
        }
    }
}

impl FileData for File {
    fn calc_size(&mut self) {}

    fn size(&self) -> u64 {
        self.size
    }

    fn largest_child(&self) -> u64 {
        self.size
    }

    fn path(&self) -> Cow<str> {
        return Cow::Borrowed(&self.path);
    }

    fn is_file(&self) -> bool {
        return true;
    }

    fn print(&self, cutoff: u64) {
        if self.size > cutoff {
            println!("{0: <8} f {1}", bytes_to_nice(self.size), self.path);
        }
    }
}

fn bytes_to_nice(bytes: u64) -> String {
    if bytes > 1024 * 1024 * 1024 {
        return format!("{} GiB", bytes / (1024 * 1024 * 1024));
    } else if bytes > 1024 * 1024 {
        return format!("{} MiB", bytes / (1024 * 1024));
    } else if bytes > 1024 {
        return format!("{} KiB", bytes / 1024);
    }
    return format!("{} B", bytes);
}
