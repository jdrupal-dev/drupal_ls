use lsp_types::Uri;
use url::Url;

pub fn uri_to_url(uri: Uri) -> Option<String> {
    Some(
        Url::parse(&uri.to_string())
            .ok()?
            .to_file_path()
            .ok()?
            .to_str()?
            .to_string(),
    )
}
