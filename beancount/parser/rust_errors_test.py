import textwrap
import unittest

from beancount.parser import parser as parser_mod


class TestRustParserErrors(unittest.TestCase):
    def parse(self, src: str):
        return parser_mod.parse_string(textwrap.dedent(src))

    def test_syntax_error_carries_precise_location(self):
        _, errors, _ = self.parse(
            """
            2014-01-01 open
            """
        )

        self.assertTrue(errors)
        self.assertEqual(2, errors[0].line)
        self.assertGreaterEqual(errors[0].column, 1)
        self.assertIsNotNone(errors[0].span_start)
        self.assertIsNotNone(errors[0].span_end)
        self.assertNotIn("Unrecognized directive", errors[0].message)


if __name__ == "__main__":
    unittest.main()
