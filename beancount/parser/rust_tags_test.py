import textwrap
import unittest

from beancount.parser import parser as parser_mod


class TestRustPushPopTags(unittest.TestCase):
    def parse(self, src: str):
        return parser_mod.parse_string(textwrap.dedent(src))

    def test_pop_invalid_tag(self):
        _, errors, _ = self.parse(
            """
            poptag #trip
            2014-01-01 open Assets:Cash
            """
        )
        self.assertTrue(errors)
        self.assertRegex(errors[0].message, "absent tag")

    def test_unbalanced_pushtag_reports_error(self):
        entries, errors, _ = self.parse(
            """
            pushtag #trip
            2014-01-01 * "Dinner"
              Assets:Cash -10 USD
              Expenses:Food 10 USD
            """
        )
        self.assertTrue(errors)
        self.assertRegex(errors[0].message, "Unbalanced pushed tag")
        self.assertEqual(2, errors[0].line)
        self.assertEqual(1, errors[0].column)
        self.assertIsNotNone(errors[0].span_start)
        self.assertIsNotNone(errors[0].span_end)
        self.assertEqual(1, len(entries))
        self.assertIn("trip", entries[0].tags)

    def test_tags_applied_until_popped(self):
        entries, errors, _ = self.parse(
            """
            pushtag #project
            2014-01-01 * "One"
              Assets:Cash -1 USD
              Expenses:Food 1 USD
            poptag #project
            2014-01-02 * "Two"
              Assets:Cash -1 USD
              Expenses:Food 1 USD
            """
        )
        self.assertFalse(errors)
        self.assertEqual(2, len(entries))
        first, second = entries
        self.assertIn("project", first.tags)
        self.assertNotIn("project", second.tags)

    def test_duplicate_pushtag_keeps_outer_tag_active(self):
        entries, errors, _ = self.parse(
            """
            pushtag #project
            pushtag #project
            2014-01-01 * "One"
              Assets:Cash -1 USD
              Expenses:Food 1 USD
            poptag #project
            2014-01-02 * "Two"
              Assets:Cash -1 USD
              Expenses:Food 1 USD
            poptag #project
            2014-01-03 * "Three"
              Assets:Cash -1 USD
              Expenses:Food 1 USD
            """
        )
        self.assertFalse(errors)
        self.assertEqual(3, len(entries))
        self.assertIn("project", entries[0].tags)
        self.assertIn("project", entries[1].tags)
        self.assertNotIn("project", entries[2].tags)


if __name__ == "__main__":
    unittest.main()
