pub mod document;

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

use ignore::overrides::OverrideBuilder;
use ignore::{WalkBuilder, WalkState};
use lsp_types::TextDocumentContentChangeEvent;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use url::Url;

use crate::parser::tokens::{PhpClassName, PhpMethod, Token, TokenData};

use self::document::{Document, FileType};

pub static DOCUMENT_STORE: LazyLock<Mutex<DocumentStore>> =
    LazyLock::new(|| Mutex::new(DocumentStore::new()));

pub fn initialize_document_store(root_dir: String) {
    log::info!("Starting project initialization...");
    let now = SystemTime::now();

    let mut builder = WalkBuilder::new(&root_dir);
    builder.standard_filters(false);

    let mut override_builder = OverrideBuilder::new(&root_dir);
    override_builder.add("**/*.services.yml").unwrap();
    override_builder.add("**/*.routing.yml").unwrap();
    override_builder.add("**/src/**/*.php").unwrap();
    override_builder.add("**/core/lib/**/*.php").unwrap();
    // For now we don't care about interfaces at all.
    override_builder.add("!**/src/**/*Interface.php").unwrap();
    override_builder.add("!**/core/lib/**/*Interface.php").unwrap();
    override_builder.add("!**/Plugin/**/*.php").unwrap();
    override_builder.add("!vendor").unwrap();
    override_builder.add("!node_modules").unwrap();
    override_builder.add("!libraries").unwrap();
    builder.overrides(override_builder.build().unwrap());

    // Find all of the documents that we are interested in parsing by walking the file tree using a
    // parallel iterator.
    let document_paths = Arc::new(Mutex::new(vec![]));
    builder.build_parallel().run(|| {
        let document_paths = document_paths.clone();
        Box::new(move |result| {
            if let Ok(dir_entry) = result {
                if dir_entry.path().is_file() {
                    document_paths
                        .lock()
                        .unwrap()
                        .push(Url::from_file_path(dir_entry.path()));
                }
            }
            WalkState::Continue
        })
    });

    // Start parsing files in parallel.
    let documents: HashMap<String, Document> = document_paths
        .lock()
        .unwrap()
        .clone()
        .into_par_iter()
        .filter_map(|path| {
            if let Ok(path) = path {
                let path = path.to_file_path().unwrap().to_str().unwrap().to_string();
                let uri = format!("file://{}", path).to_string();
                let text = fs::read_to_string(&path).unwrap();

                let mut document = Document::new(&uri, text);
                document.parse();
                return Some((uri, document));
            }
            None
        })
        .collect();

    log::info!(
        "Parsed {} files in {} seconds",
        documents.len(),
        now.elapsed().unwrap().as_secs()
    );

    DOCUMENT_STORE.lock().unwrap().add_documents(documents);
}

pub struct DocumentStore {
    documents: Box<HashMap<String, Document>>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: Box::new(HashMap::new()),
        }
    }

    pub fn get_document(&mut self, uri: &String) -> Option<&Document> {
        self.documents.get(uri)
    }

    fn get_document_mut(&mut self, uri: &String) -> Option<&mut Document> {
        self.documents.get_mut(uri)
    }

    pub fn get_documents(&self) -> &HashMap<String, Document> {
        &self.documents
    }

    pub fn add_document(&mut self, uri: &String, text: String) {
        self.documents
            .insert(uri.to_string(), Document::new(uri, text));
        let document = self.get_document_mut(uri).unwrap();
        document.parse();
    }

    pub fn add_documents(&mut self, documents: HashMap<String, Document>) {
        self.documents.extend(documents);
    }

    pub fn change_document(&mut self, uri: &String, changes: Vec<TextDocumentContentChangeEvent>) {
        if changes.len() > 1 {
            log::error!(
                "Only full text document sync is supported! Received {} content changes for {}",
                changes.len(),
                uri
            );
            return;
        }

        match self.get_document_mut(uri) {
            Some(document) => {
                for change in changes {
                    document.set_content(change.text);
                }
                document.parse();
            }
            None => log::error!("Unable to apply changes to non-existing document: {}", uri),
        }
    }

    // TODO: Consider moving this to a separate module.
    pub fn get_service_definition(&self, service_name: &str) -> Option<(&Document, &Token)> {
        let files = self.get_documents_by_file_type(FileType::Yaml);

        files.iter().find_map(|&document| {
            Some((
                document,
                document.tokens.iter().find(|token| {
                    if let TokenData::DrupalServiceDefinition(service) = &token.data {
                        return service.name == service_name;
                    }
                    return false;
                })?,
            ))
        })
    }

    pub fn get_route_definition(&self, route_name: &str) -> Option<(&Document, &Token)> {
        let files = self.get_documents_by_file_type(FileType::Yaml);

        files.iter().find_map(|&document| {
            Some((
                document,
                document.tokens.iter().find(|token| {
                    if let TokenData::DrupalRouteDefinition(route) = &token.data {
                        return route.name == route_name;
                    }
                    return false;
                })?,
            ))
        })
    }

    pub fn get_class_definition(&self, class_name: &PhpClassName) -> Option<(&Document, &Token)> {
        let files = self.get_documents_by_file_type(FileType::Php);

        files.iter().find_map(|&document| {
            Some((
                document,
                document.tokens.iter().find(|token| {
                    if let TokenData::PhpClassDefinition(class) = &token.data {
                        return class.name == *class_name;
                    }
                    return false;
                })?,
            ))
        })
    }

    pub fn get_method_definition(&self, method: &PhpMethod) -> Option<(&Document, &Token)> {
        if let Some((document, token)) = self.get_class_definition(&method.class_name) {
            if let TokenData::PhpClassDefinition(class) = &token.data {
                let token = class.methods.get(&method.name)?;
                return Some((document, token));
            }
        }
        None
    }

    pub fn get_hook_definition(&self, hook_name: &str) -> Option<(&Document, &Token)> {
        let files = self.get_documents_by_file_type(FileType::Php);

        files.iter().find_map(|&document| {
            Some((
                document,
                document.tokens.iter().find(|token| {
                    if let TokenData::DrupalHookDefinition(hook) = &token.data {
                        return hook.name == hook_name;
                    }
                    return false;
                })?,
            ))
        })
    }

    fn get_documents_by_file_type(&self, file_type: FileType) -> Vec<&Document> {
        self.documents
            .values()
            .filter(|document| document.file_type == file_type)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::TextDocumentContentChangeEvent;

    use crate::document_store::document::FileType;
    use crate::document_store::DocumentStore;

    #[test]
    fn add_document_to_store() {
        let mut store = DocumentStore::new();

        let test_document = String::from("This is a test document.");
        let test_uri = String::from("file://test.php");
        store.add_document(&test_uri, test_document.clone());

        assert_eq!(
            test_document,
            store.get_document(&test_uri).unwrap().content
        );
        assert_eq!(
            FileType::Php,
            store.get_document(&test_uri).unwrap().file_type
        );
    }

    #[test]
    fn change_document_in_store() {
        let mut store = DocumentStore::new();

        let test_uri = String::from("file://test-file.txt");
        store.add_document(&test_uri, String::new());

        let updated_document = String::from("This is an updated document.");
        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: updated_document.clone(),
        }];
        store.change_document(&test_uri, changes);

        assert_eq!(
            updated_document,
            store.get_document(&test_uri).unwrap().content
        );
    }
}
