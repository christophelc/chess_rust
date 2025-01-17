use std::fs;

use crate::{
    benchmark::epd_reader::{self, EpdRead},
    ui::notation::{epd, san},
};

use super::epd_reader::EpdFileReaderError;

fn list_epd_files_in_folder(epd_folder: &str) -> Result<Vec<String>, std::io::Error> {
    let mut epd_files = vec![];

    // Read the directory
    for entry in fs::read_dir(epd_folder)? {
        let entry = entry?;
        let path = entry.path();
        // Check if the file has a ".epd" extension

        if let Some(extension) = path.extension() {
            if extension == "epd" {
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        epd_files.push(file_name_str.to_string());
                    }
                }
            }
        }
    }

    Ok(epd_files)
}

#[derive(Debug)]
pub struct EpdData {
    folder: String,
    epds: Vec<epd::Epd>,
}
impl std::fmt::Display for EpdData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let epds_as_str: Vec<String> = self.epds.iter().map(|epd| epd.to_string()).collect();
        let separator = "------------------------";
        write!(
            f,
            "{}{}{}\n{}\n{}",
            separator,
            self.folder,
            separator,
            epds_as_str.join("\n"),
            separator
        )
    }
}
impl EpdData {
    pub fn new(folder: String, epds: Vec<epd::Epd>) -> Self {
        Self { folder, epds }
    }
}
pub fn benchmark(epd_folder: &str) -> Result<Vec<EpdData>, EpdFileReaderError> {
    let data_all_files_or_error = read_epds_from_folder(epd_folder);
    if let Ok(data_all_files) = &data_all_files_or_error {
        for data_per_file in data_all_files {
            println!("{}", data_per_file.to_string());
        }
    }
    data_all_files_or_error
}
pub fn read_epds_from_folder(epd_folder: &str) -> Result<Vec<EpdData>, EpdFileReaderError> {
    let lang = &san::Lang::LangEn;
    let mut v: Vec<EpdData> = vec![];
    let files_str = list_epd_files_in_folder(epd_folder)?;

    for file_str in files_str.iter() {
        let file = format!("{}/{}", epd_folder, file_str);
        let epd_file = epd_reader::EpdFile(file.clone());
        let epds = epd_file.epd_read(lang)?;
        v.push(EpdData::new(file, epds));
    }
    Ok(v)
}
