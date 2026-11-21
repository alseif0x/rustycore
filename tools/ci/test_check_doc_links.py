"""Hermetic regression tests for repository Markdown link resolution."""

from pathlib import Path
import tempfile
import unittest

import check_doc_links


class DocumentLinkTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def write(self, name: str, text: str = "") -> Path:
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        return path

    def test_inline_and_multiline_code_spans_do_not_create_links(self):
        document = self.write(
            "docs/index.md",
            "`[inline](missing-inline.md)`\n"
            "``example `[double](missing-double.md)` code``\n"
            "Before ``a multiline code span\n"
            "[multiline](missing-multiline.md)\n"
            "`` after [valid](target.md)\n",
        )
        self.write("docs/target.md")

        self.assertEqual(check_doc_links.broken_links(document), [])

    def test_escaped_backtick_does_not_hide_a_real_link(self):
        document = self.write("guide.md", r"\`[visible](missing.md)" + "\n")

        errors = check_doc_links.broken_links(document)
        self.assertEqual(len(errors), 1)
        self.assertIn("missing.md", errors[0])

    def test_extensionless_vitepress_routes_resolve_markdown_and_index_files(self):
        document = self.write(
            "docs/wiki/client/index.md",
            "[Setup](./setup)\n"
            "[Server setup](../server/setup?mode=local#step-one)\n"
            "[Reference](../reference/)\n",
        )
        self.write("docs/wiki/client/setup.md")
        self.write("docs/wiki/server/setup.md")
        self.write("docs/wiki/reference/index.md")

        self.assertEqual(check_doc_links.broken_links(document), [])

    def test_missing_extensionless_route_stays_a_failure(self):
        document = self.write("docs/wiki/index.md", "[Missing](./not-published)\n")

        errors = check_doc_links.broken_links(document)
        self.assertEqual(len(errors), 1)
        self.assertIn("not-published", errors[0])

    def test_fenced_code_is_still_ignored(self):
        document = self.write("guide.md", "```md\n[example](missing.md)\n```\n")

        self.assertEqual(check_doc_links.broken_links(document), [])


if __name__ == "__main__":
    unittest.main()
