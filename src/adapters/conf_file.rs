use std::{collections::HashMap, fs};

use hashlink::LinkedHashMap;
use yaml_rust2::Yaml;

use super::raikirifs::ThreadSafeError;

static CONF_FILE_PATH: &str = "raikiri.yaml";

#[derive(Clone)]
pub struct ConfFile {
    pub components: HashMap<String, String>,
    pub run_confs: HashMap<String, RunConf>,
}

impl ConfFile {
    pub fn build() -> Result<ConfFile, ThreadSafeError> {
        let content = fs::read_to_string(CONF_FILE_PATH).unwrap();
        let content = yaml_rust2::YamlLoader::load_from_str(&content)?;
        let content = content
            .get(0).unwrap()
            .as_hash().unwrap();

        let yaml_str = |arg: &'static str| Yaml::String(arg.to_string());

        let file_components = content.get(&yaml_str("components")).unwrap().as_hash().unwrap();
        let file_run_confs = content.get(&yaml_str("run")).unwrap().as_hash().unwrap();

        let mut components = HashMap::new();
        for (k, v) in file_components.iter() {
            components.insert(k.as_str().unwrap().to_string(), v.as_str().unwrap().to_string());
        }

        let mut run_confs = HashMap::new();
        for (k, v) in file_run_confs.iter() {
            let conf = v.as_hash().unwrap();
            let headers = conf.get(&yaml_str("component")).unwrap().as_hash().unwrap();
            run_confs.insert(k.as_str().unwrap().to_string(), RunConf {
                component: conf.get(&yaml_str("component")).unwrap().as_str().unwrap().to_string(),
                method: conf.get(&yaml_str("method")).unwrap().as_str().unwrap().to_string(),
                headers: headers.clone(),
                body: conf.get(&yaml_str("component")).unwrap().as_str().unwrap().to_string()
            });
        }

        Ok(ConfFile {
            components,
            run_confs,
        })
    }
}

#[derive(Clone)]
pub struct RunConf {
    pub component: String,
    pub method: String,
    pub headers: LinkedHashMap<Yaml, Yaml>,
    pub body: String
}