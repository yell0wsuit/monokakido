use miniserde::{json, Deserialize};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};
use toml::Value;

use crate::{key::Keys, media::Media, pages::Pages, Error};

pub struct MonokakidoDict {
    paths: Paths,
    pub pages: Pages,
    pub audio: Option<Media>,
    pub graphics: Option<Media>,
    pub keys: Keys,
}

#[derive(Deserialize, Debug)]
struct DictJson {
    #[serde(rename = "DSProductContents")]
    contents: Vec<DSProductContents>,
}

#[derive(Deserialize, Debug)]
struct DSProductContents {
    #[serde(rename = "DSContentDirectory")]
    dir: String,
}

pub struct Paths {
    base_path: PathBuf,
    name: String,
    contents_dir: String,
}

impl AsRef<Path> for Paths {
    fn as_ref(&self) -> &Path {
        &self.base_path
    }
}

impl Paths {
    fn read_config() -> Result<String, Box<dyn std::error::Error>> {
        let config_str = fs::read_to_string("config.toml")?;
        let config: Value = toml::from_str(&config_str)?;

        let dict_path = config["dict_path"]
            .as_str()
            .ok_or("dict_path not found or not a string")?
            .to_string();

        Ok(dict_path)
    }

    fn list_path(custom_dir: Option<&str>) -> PathBuf {
        if let Some(dir) = custom_dir {
            return PathBuf::from(dir);
        }

        Self::read_config().map(PathBuf::from).unwrap_or_else(|_| {
            // Fallback to default path if config reading fails
            PathBuf::from(
                "/Library/Application Support/AppStoreContent/jp.monokakido.Dictionaries/Products/",
            )
        })
    }

    fn std_dict_path(name: &str, custom_dir: Option<&str>) -> PathBuf {
        let mut path = Paths::list_path(custom_dir);
        path.push(format!("{name}"));
        path
    }

    fn json_path(path: &Path, name: &str) -> PathBuf {
        let mut pb = PathBuf::from(path);
        pb.push("Contents");
        pb.push(format!("{name}.json"));
        pb
    }

    pub(crate) fn contents_path(&self) -> PathBuf {
        let mut pb = PathBuf::from(&self.base_path);
        pb.push("Contents");
        pb.push(&self.contents_dir);
        pb
    }

    pub(crate) fn key_path(&self) -> PathBuf {
        let mut pb = self.contents_path();
        pb.push("key");
        pb
    }

    pub(crate) fn key_headword_path(&self) -> PathBuf {
        let mut pb = self.key_path();
        pb.push("headword.keystore");
        pb
    }

    pub(crate) fn headline_path(&self) -> PathBuf {
        let mut pb = self.contents_path();
        pb.push("headline");
        pb
    }

    pub(crate) fn headline_long_path(&self) -> PathBuf {
        let mut pb = self.headline_path();
        pb.push("headline.headlinestore");
        pb
    }
}

fn parse_dict_name(fname: &OsStr) -> Option<&str> {
    let fname = fname.to_str()?;
    let dict_prefix = "";
    fname.strip_prefix(dict_prefix)
}

impl MonokakidoDict {
    pub fn list_with_dir(
        custom_dir: Option<&str>,
    ) -> Result<impl Iterator<Item = Result<String, Error>>, Error> {
        let iter = fs::read_dir(Paths::list_path(custom_dir)).map_err(|_| Error::IOError)?;
        Ok(iter.filter_map(|entry| {
            entry
                .map_err(|_| Error::IOError)
                .map(|e| parse_dict_name(&e.file_name()).map(ToOwned::to_owned))
                .transpose()
        }))
    }

    pub fn list() -> Result<impl Iterator<Item = Result<String, Error>>, Error> {
        Self::list_with_dir(None)
    }

    pub fn open_with_dir(name: &str, custom_dir: Option<&str>) -> Result<Self, Error> {
        let std_path = Paths::std_dict_path(name, custom_dir);
        Self::open_with_path_name(std_path, name)
    }

    pub fn open(name: &str) -> Result<Self, Error> {
        Self::open_with_dir(name, None)
    }

    pub fn name(&self) -> &str {
        &self.paths.name
    }

    pub fn open_with_path(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let path: PathBuf = path.into();
        let dir_name = path.file_name().ok_or(Error::FopenError)?.to_string_lossy();

        let dict_name = dir_name.rsplit_once('.').ok_or(Error::FopenError)?.0;

        Self::open_with_path_name(&path, dict_name)
    }

    fn open_with_path_name(path: impl Into<PathBuf>, name: &str) -> Result<Self, Error> {
        let base_path = path.into();
        let json_path = Paths::json_path(&base_path, name);
        println!("DEBUG: Reading JSON from: {:?}", json_path);
        let json = fs::read_to_string(json_path).map_err(|_| Error::NoDictJsonFound)?;
        let mut json: DictJson = json::from_str(&json).map_err(|_| Error::InvalidDictJson)?;
        let contents = json.contents.pop().ok_or(Error::InvalidDictJson)?;
        let paths = Paths {
            base_path,
            name: name.to_owned(),
            contents_dir: contents.dir,
        };

        println!("DEBUG: Contents directory: {:?}", paths.contents_path());

        println!("DEBUG: Initializing Pages...");
        let pages = Pages::new(&paths)?;

        println!("DEBUG: Initializing Media (audio)...");
        let audio = Media::new(&paths)?;

        println!("DEBUG: Initializing Media (graphics)...");
        let graphics = Media::new(&paths)?;

        println!("DEBUG: Initializing Keys...");
        let keys = Keys::new(paths.key_headword_path())?;

        println!("DEBUG: All components initialized successfully!");

        Ok(MonokakidoDict {
            paths,
            pages,
            audio,
            graphics,
            keys,
        })
    }
}
