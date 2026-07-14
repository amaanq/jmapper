//! Canonicalize DAV hrefs before storing or comparing them.
//!
//! Servers return absolute URIs, paths, and inconsistent percent-encoding.

use percent_encoding::{
   AsciiSet,
   CONTROLS,
   percent_decode_str,
   utf8_percent_encode,
};

/// Bytes that must stay encoded inside a path segment. Everything else is
/// decoded to its literal form — the canonical representation prefers
/// decoded characters so two encodings of the same segment collide.
const SEGMENT: &AsciiSet = &CONTROLS
   .add(b' ')
   .add(b'"')
   .add(b'#')
   .add(b'%')
   .add(b'<')
   .add(b'>')
   .add(b'?')
   .add(b'/')
   .add(b'\\')
   .add(b'^')
   .add(b'`')
   .add(b'{')
   .add(b'|')
   .add(b'}');

/// Canonicalize an href to an absolute path: strips scheme+authority,
/// re-encodes each segment minimally, collapses duplicate slashes, and
/// preserves a trailing slash (the collection marker).
#[inline]
pub fn normalize_href(href: &str) -> String {
   // Absolute URI → keep only the path. `url::Url` handles scheme quirks.
   let path = if href.starts_with("http://") || href.starts_with("https://") {
      url::Url::parse(href).map_or_else(|_| href.to_owned(), |url| url.path().to_owned())
   } else {
      // Strip any query/fragment — collection hrefs never carry them and
      // resource hrefs with them are server bugs we normalize away.
      let end = href.find(['?', '#']).unwrap_or(href.len());
      href[..end].to_owned()
   };

   let trailing_slash = path.ends_with('/') && path.len() > 1;
   let mut out = String::with_capacity(path.len() + 1);
   for segment in path.split('/').filter(|text| !text.is_empty()) {
      let decoded = percent_decode_str(segment)
         .decode_utf8()
         .map_or_else(|_| segment.to_owned(), std::borrow::Cow::into_owned);
      out.push('/');
      out.push_str(&utf8_percent_encode(&decoded, SEGMENT).to_string());
   }
   if out.is_empty() || trailing_slash {
      out.push('/');
   }
   out
}

/// Resolve a possibly-relative href against a base collection path.
#[must_use]
#[inline]
pub fn resolve_href(base: &str, href: &str) -> String {
   if href.starts_with('/') || href.starts_with("http://") || href.starts_with("https://") {
      return normalize_href(href);
   }
   let base_path = normalize_href(base);
   let directory = base_path
      .rfind('/')
      .map_or("/", |index| &base_path[..=index]);
   normalize_href(&format!("{directory}{href}"))
}

/// The final path segment, percent-decoded — the human-ish resource name.
#[inline]
pub fn last_segment(href: &str) -> String {
   let norm = normalize_href(href);
   let trimmed = norm.trim_end_matches('/');
   let seg = trimmed.rsplit('/').next().unwrap_or("");
   percent_decode_str(seg)
      .decode_utf8()
      .map_or_else(|_| seg.to_owned(), std::borrow::Cow::into_owned)
}

#[cfg(test)]
mod tests {
   use pretty_assertions::assert_eq;

   use super::*;

   #[test]
   fn strips_scheme_and_authority() {
      assert_eq!(
         normalize_href("https://dav.example.com:8443/cal/user/"),
         "/cal/user/"
      );
      assert_eq!(normalize_href("/cal/user/"), "/cal/user/");
   }

   #[test]
   fn canonicalizes_percent_encoding() {
      // Same segment, three spellings.
      assert_eq!(normalize_href("/cal/a%20b.ics"), "/cal/a%20b.ics");
      assert_eq!(normalize_href("/cal/a b.ics"), "/cal/a%20b.ics");
      assert_eq!(normalize_href("/cal/%61%20b.ics"), "/cal/a%20b.ics");
      // Unreserved characters decode.
      assert_eq!(normalize_href("/c%61l/x.ics"), "/cal/x.ics");
   }

   #[test]
   fn collapses_doubled_slashes_and_keeps_collection_marker() {
      assert_eq!(normalize_href("//cal//user//"), "/cal/user/");
      assert_eq!(normalize_href("/cal/user"), "/cal/user");
      assert_eq!(normalize_href(""), "/");
      assert_eq!(normalize_href("/"), "/");
   }

   #[test]
   fn drops_query_and_fragment() {
      assert_eq!(normalize_href("/cal/x.ics?export"), "/cal/x.ics");
      assert_eq!(normalize_href("/cal/x.ics#frag"), "/cal/x.ics");
   }

   #[test]
   fn resolves_relative() {
      assert_eq!(
         resolve_href("/cal/user/", "event.ics"),
         "/cal/user/event.ics"
      );
      assert_eq!(resolve_href("/cal/user", "event.ics"), "/cal/event.ics");
      assert_eq!(
         resolve_href("/cal/user/", "https://x.example/other/e.ics"),
         "/other/e.ics"
      );
   }

   #[test]
   fn last_segment_decodes() {
      assert_eq!(last_segment("/cal/user/a%20b.ics"), "a b.ics");
      assert_eq!(last_segment("/cal/user/"), "user");
   }
}
