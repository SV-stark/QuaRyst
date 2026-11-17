use tantivy::schema::{
    FAST, INDEXED, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions,
};
use tantivy::tokenizer::{TextAnalyzer, Token, TokenStream, Tokenizer};

/// Name of the custom word tokenizer registered on every `QuaRyst` index.
pub const WORD_TOKENIZER: &str = "quaryst_words";

/// Version of the on-disk layout.
///
/// Bump this whenever `create_schema` changes in a way that makes an existing
/// index unreadable or inconsistent. `IndexManager` compares it against the
/// stored value and rotates the old index aside for a clean rebuild rather than
/// trying to migrate documents in place.
///
/// 3.0.0: `content` is no longer STORED. Snippets are re-extracted from disk.
/// 4.0.0: `content_preview` (first ~4 KB, STORED) added so snippets come
///        from the index instead of a per-query disk re-read. Falls back to
///        disk extraction when the match is beyond the preview window.
/// 5.0.0: Custom `quaryst_words` tokenizer (CJK char-level tokens, skips
///        >40-char words and >12-digit runs to keep the term dictionary compact).
pub const SCHEMA_LAYOUT_VERSION: &str = "5.0.0";

const fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x3040..=0x30FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0xAC00..=0xD7AF
            | 0x20000..=0x2FFFF
    )
}

/// Splits text into lowercase words; CJK characters become one token each.
/// Very long words (>40 chars) and long digit runs (>12 digits) are skipped
/// to keep the FST term dictionary compact.
#[derive(Clone, Default)]
pub struct WordTokenizer {
    token: Token,
}

pub struct WordStream<'a> {
    text: &'a str,
    pos: usize,
    token: &'a mut Token,
}

impl Tokenizer for WordTokenizer {
    type TokenStream<'a> = WordStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> WordStream<'a> {
        self.token.text.clear();
        self.token.position = usize::MAX;
        self.token.position_length = 1;
        self.token.offset_from = 0;
        self.token.offset_to = 0;
        WordStream {
            text,
            pos: 0,
            token: &mut self.token,
        }
    }
}

impl TokenStream for WordStream<'_> {
    fn advance(&mut self) -> bool {
        loop {
            let rest = &self.text[self.pos..];
            let Some((i, c)) = rest.char_indices().find(|(_, c)| c.is_alphanumeric()) else {
                self.pos = self.text.len();
                return false;
            };
            let start = self.pos + i;
            let t = &mut *self.token;
            t.text.clear();
            if is_cjk(c) {
                let end = start + c.len_utf8();
                t.text.push(c);
                self.pos = end;
                t.offset_from = start;
                t.offset_to = end;
            } else {
                let mut end = self.text.len();
                let mut chars = 0usize;
                for (k, ch) in self.text[start..].char_indices() {
                    if !ch.is_alphanumeric() || is_cjk(ch) {
                        end = start + k;
                        break;
                    }
                    chars += 1;
                    for l in ch.to_lowercase() {
                        t.text.push(l);
                    }
                }
                self.pos = end;
                let digits = t.text.len() > 12 && t.text.bytes().all(|b| b.is_ascii_digit());
                if chars > 40 || digits {
                    continue;
                }
                t.offset_from = start;
                t.offset_to = end;
            }
            t.position = t.position.wrapping_add(1);
            return true;
        }
    }

    fn token(&self) -> &Token {
        self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        self.token
    }
}

/// Calls `emit` with each lowercase word of `text` and its byte range.
pub fn words_with_offsets(text: &str, mut emit: impl FnMut(&str, usize, usize) -> bool) {
    let mut tk = WordTokenizer::default();
    let mut st = tk.token_stream(text);
    while st.advance() {
        let t = st.token();
        if !emit(&t.text, t.offset_from, t.offset_to) {
            break;
        }
    }
}

/// Split text into lowercase word tokens. CJK characters become one token each.
pub fn tokenize_words(text: &str, mut emit: impl FnMut(&str)) {
    words_with_offsets(text, |w, _, _| {
        emit(w);
        true
    });
}

/// Registers `QuaRyst`'s custom tokenizers on an opened/created Tantivy `Index`.
pub fn register_tokenizers(index: &tantivy::Index) {
    index.tokenizers().register(
        WORD_TOKENIZER,
        TextAnalyzer::builder(WordTokenizer::default()).build(),
    );
}

/// Create Tantivy schema optimized for file search with minimal posting list overhead
#[must_use]
pub fn create_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // File path - stored for retrieval, indexed as string for fast term deletes
    schema_builder.add_text_field("file_path", STRING | STORED);

    // Content - indexed full-text stream.
    //
    // NOT stored. The field previously carried `.set_stored()`, which wrote
    // every extracted document body into the index's doc store. That made the
    // on-disk index grow with the total corpus size and forced a full
    // decompress of each hit's entire body just to produce a one-line snippet.
    // At the default `max_results` of 10_000 that is a serious I/O cliff.
    //
    // Snippets are now re-extracted from the file on demand (see
    // `crate::snippet`), and the indexed stream carries positions
    // (`WithFreqsAndPositions`) precisely so that on-disk extraction can still
    // locate and highlight the match.
    let text_options = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer(WORD_TOKENIZER)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    schema_builder.add_text_field("content", text_options);

    // Content preview — first ~4 KB of the extracted text, STORED only.
    //
    // `content` above is indexed but not stored, so producing a snippet used
    // to require re-reading and re-parsing every matching file from disk
    // (Phase B), which dominated query latency by 4-5 orders of magnitude.
    // Storing a bounded excerpt here lets `searcher` render snippets from
    // memory; disk extraction remains as a fallback when the match falls
    // outside the preview window.
    let preview_options = TextOptions::default().set_stored();
    schema_builder.add_text_field("content_preview", preview_options);

    // Title - stored for display, indexed for search
    let title_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(WORD_TOKENIZER)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();
    schema_builder.add_text_field("title", title_options);

    // Modified timestamp - fast field for sorting & date range queries
    schema_builder.add_date_field("modified", FAST | INDEXED);

    // File size - fast field for range queries
    schema_builder.add_u64_field("size", FAST | INDEXED);

    // File extension - keyword indexed for exact ext filter.
    // STORED so results can display the correct file-type badge/icon; without it
    // `retrieve_result_with_doc` always reads back `None` and the UI falls back
    // to a generic "FILE" badge for every result.
    schema_builder.add_text_field("extension", STRING | STORED);

    schema_builder.build()
}
