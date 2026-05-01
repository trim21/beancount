import textwrap
import unittest

from beancount.parser import parser as parser_mod


class TestRustParserErrors(unittest.TestCase):
    def parse(self, src: str):
        return parser_mod.parse_string(textwrap.dedent(src))

    def test_syntax_error_carries_precise_location(self):
        _, errors, _ = self.parse(
            """
            not-a-directive
            """
        )

        self.assertTrue(errors)
        self.assertEqual(2, errors[0].line)
        self.assertGreaterEqual(errors[0].column, 1)
        self.assertEqual("syntax", errors[0].kind)
        self.assertEqual("syntax error", errors[0].reason)
        self.assertEqual("<string>", errors[0].source_id)
        self.assertIsNotNone(errors[0].span_start)
        self.assertIsNotNone(errors[0].span_end)
        self.assertIsNotNone(errors[0].span)
        self.assertEqual(2, errors[0].span.line)
        self.assertIn("not-a-directive", errors[0].span.excerpt)
        self.assertEqual([], errors[0].contexts)
        self.assertEqual([], errors[0].related)
        self.assertNotIn("Unrecognized directive", errors[0].message)

    def test_semantic_error_gets_best_effort_span(self):
        _, errors, _ = self.parse(
            """
            2014-13-01 open Assets:Cash
            """
        )

        self.assertTrue(errors)
        self.assertEqual("semantic", errors[0].kind)
        self.assertEqual("directive normalization error", errors[0].reason)
        self.assertIsNotNone(errors[0].span)
        self.assertIn("2014-13-01 open Assets:Cash", errors[0].span.excerpt)


if __name__ == "__main__":
    unittest.main()
