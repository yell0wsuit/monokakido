use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use monokakido::{Error, MonokakidoDict};

fn print_help() {
    println!("Monokakido CLI. Supported subcommands:");
    println!("  --dir <path>  Use custom dictionary directory (optional, before subcommand)");
    println!("  list          Lists all dictionaries installed in the standard path");
    println!("  list_items <dict> <keyword>   Lists all items");
    println!("  list_audio <dict> <keyword>   Lists all audio files");
    println!("  get_audio <dict> <id>         Writes an audio file to stdout");
    println!("  dump <dict>   Dumps all dictionary entries in XML format");
    println!("  help          This help");
}

fn parse_args() -> (Option<String>, Vec<String>) {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0); // Remove program name

    let mut custom_dir = None;
    let mut remaining_args = Vec::new();

    let mut i = 0;
    while i < args.len() {
        if args[i] == "--dir" && i + 1 < args.len() {
            custom_dir = Some(args[i + 1].clone());
            i += 2; // Skip both --dir and its value
        } else {
            remaining_args.push(args[i].clone());
            i += 1;
        }
    }

    (custom_dir, remaining_args)
}

fn list_items(dict_name: &str, keyword: &str, custom_dir: Option<&str>) -> Result<(), Error> {
    let mut dict = MonokakidoDict::open_with_dir(dict_name, custom_dir)?;
    let (_, items) = dict.keys.search_exact(keyword)?;

    for id in items {
        let item = dict.pages.get_item(id)?;
        println!("{item}");
    }
    Ok(())
}

fn list_pages(dict_name: &str, keyword: &str, custom_dir: Option<&str>) -> Result<(), Error> {
    let mut dict = MonokakidoDict::open_with_dir(dict_name, custom_dir)?;
    let (_, items) = dict.keys.search_exact(keyword)?;

    for id in items {
        let page = dict.pages.get_page(id)?;
        println!("{page}");
    }
    Ok(())
}

fn list_audio(dict_name: &str, keyword: &str, custom_dir: Option<&str>) -> Result<(), Error> {
    let mut dict = MonokakidoDict::open_with_dir(dict_name, custom_dir)?;
    let (_, items) = dict.keys.search_exact(keyword)?;

    for id in items {
        for audio in dict.pages.get_item_audio(id)? {
            if let Some((_, audio)) = audio?.split_once("href=\"") {
                if let Some((id, _)) = audio.split_once('"') {
                    println!("{id}");
                }
            }
        }
    }
    Ok(())
}

fn get_audio(dict_name: &str, id: &str, custom_dir: Option<&str>) -> Result<(), Error> {
    let id = id.strip_suffix(".aac").unwrap_or(id);
    let mut dict = MonokakidoDict::open_with_dir(dict_name, custom_dir)?;
    let aac = dict.audio.as_mut().ok_or(Error::MissingAudio)?.get(id)?;
    let mut stdout = std::io::stdout().lock();
    // TODO: for ergonomics/failsafe, check if stdout is a TTY
    stdout.write_all(aac)?;
    Ok(())
}

fn list_dicts(custom_dir: Option<&str>) -> Result<(), Error> {
    for dict in MonokakidoDict::list_with_dir(custom_dir)? {
        println!("{}", dict?);
    }
    Ok(())
}

fn dump_dict(dict_name: &str, custom_dir: Option<&str>) -> Result<(), Error> {
    let mut dict = MonokakidoDict::open_with_dir(dict_name, custom_dir)?;

    // Create output directory
    let output_dir = if let Some(base_dir) = custom_dir {
        Path::new(base_dir).join("outputxml")
    } else {
        Path::new("outputxml").to_path_buf()
    };

    // Ensure output directory exists
    fs::create_dir_all(&output_dir).map_err(|_| Error::IOError)?;

    // Create output file
    let output_file_path = output_dir.join(format!("{}_dump.xml", dict_name));
    let mut output_file = File::create(&output_file_path).map_err(|_| Error::IOError)?;

    println!("Dumping {} to: {}", dict_name, output_file_path.display());

    // Write XML header
    writeln!(output_file, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(
        output_file,
        r#"<d:dictionary xmlns:d="http://www.apple.com/DTDs/DictionaryService-1.0.rng">"#
    )?;

    // Iterate through all pages
    let idx_range = dict.pages.idx_iter()?;
    let total_pages = idx_range.len();
    let mut processed = 0;

    for idx in idx_range {
        let (page_id, page_xml) = dict.pages.page_by_idx(idx)?;

        // Write raw entry with minimal wrapper
        write_raw_entry(&mut output_file, page_id, page_xml)?;

        processed += 1;
        if processed % 1000 == 0 {
            println!("Processed {}/{} entries...", processed, total_pages);
        }
    }

    // Write XML footer
    writeln!(output_file, r#"</d:dictionary>"#)?;

    println!("Dump completed! Saved to: {}", output_file_path.display());
    println!("Total entries processed: {}", processed);

    Ok(())
}

fn write_raw_entry(file: &mut File, page_id: u32, page_xml: &str) -> Result<(), Error> {
    // Just wrap the raw content in a simple d:entry with page_id
    writeln!(
        file,
        r#"<d:entry id="{}" d:title="entry_{}">"#,
        page_id, page_id
    )?;

    // Write the raw XML content as-is
    writeln!(file, "{}", page_xml)?;

    writeln!(file, "</d:entry>")?;

    Ok(())
}

fn main() {
    let (custom_dir, args) = parse_args();
    let custom_dir_ref = custom_dir.as_deref();

    let res = match args.get(0).map(|s| s.as_str()) {
        Some("list_audio") => {
            if let (Some(dict_name), Some(keyword)) = (args.get(1), args.get(2)) {
                list_audio(dict_name, keyword, custom_dir_ref)
            } else {
                Err(Error::InvalidArg)
            }
        }
        Some("get_audio") => {
            if let (Some(dict_name), Some(id)) = (args.get(1), args.get(2)) {
                get_audio(dict_name, id, custom_dir_ref)
            } else {
                Err(Error::InvalidArg)
            }
        }
        Some("list_items") => {
            if let (Some(dict_name), Some(keyword)) = (args.get(1), args.get(2)) {
                list_items(dict_name, keyword, custom_dir_ref)
            } else {
                Err(Error::InvalidArg)
            }
        }
        Some("list_pages") => {
            if let (Some(dict_name), Some(keyword)) = (args.get(1), args.get(2)) {
                list_pages(dict_name, keyword, custom_dir_ref)
            } else {
                Err(Error::InvalidArg)
            }
        }
        Some("list") => list_dicts(custom_dir_ref),
        Some("dump") => {
            if let Some(dict_name) = args.get(1) {
                dump_dict(dict_name, custom_dir_ref)
            } else {
                Err(Error::InvalidArg)
            }
        }
        None | Some("help") => {
            print_help();
            Ok(())
        }
        _ => Err(Error::InvalidSubcommand),
    };

    if let Err(e) = res {
        eprintln!("Error: {e:?}");
        std::process::exit(1)
    }
}
