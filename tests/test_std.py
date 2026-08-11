"""The standard library and the native primitives it builds on."""

import unittest

from helpers import out_of, run


class TestPrimitives(unittest.TestCase):
    def test_type(self):
        self.assertEqual(out_of("5⍙⍞⍞"), "int\n5\n")  # non-destructive
        self.assertEqual(
            out_of("2.5⍙⇅⌫⍞ «x»⍙⇅⌫⍞ ⟨⟩⍙⇅⌫⍞ [∂]⍙⇅⌫⍞ ∅⍙⇅⌫⍞"),
            "float\nstr\nlist\nquot\n∅\n",
        )

    def test_reverse(self):
        self.assertEqual(out_of("«matrix»⌽⍞ ⟨1 2 3⟩⌽⍞ «»⌽⍞"), "xirtam\n⟨3 2 1⟩\n\n")

    def test_sort(self):
        self.assertEqual(out_of("⟨3 1 2⟩⍋⍞ «cba»⍋⍞ ⟨«b» «a»⟩⍋⍞"),
                         "⟨1 2 3⟩\nabc\n⟨«a» «b»⟩\n")
        self.assertEqual(out_of("⟨3 1.5 2⟩⍋⍞"), "⟨1.5 2 3⟩\n")

    def test_sort_mixed_glitches(self):
        code, _, err = run("⟨1 «a»⟩⍋")
        self.assertEqual(code, 1)
        self.assertIn("all numbers or all strings", err)

    def test_contains_find(self):
        self.assertEqual(out_of("«wake up»«ake»∈⍞ «abc»«z»∈⍞"), "1\n0\n")
        self.assertEqual(out_of("⟨1 ⟨2⟩⟩⟨2⟩∈⍞ ⟨1⟩ 9∈⍞"), "1\n0\n")
        self.assertEqual(out_of("«abcd»«cd»⍷⍞ «ab»«z»⍷⍞ ⟨7 8⟩8⍷⍞ ⟨⟩1⍷⍞"),
                         "2\n¯1\n1\n¯1\n")

    def test_search_string_with_non_string_glitches(self):
        code, _, err = run("«ab»5∈")
        self.assertEqual(code, 1)
        self.assertIn("needs a string", err)


class TestStdLibrary(unittest.TestCase):
    def test_constants(self):
        self.assertEqual(out_of("π⍞ τ π 2×=⍞ ℯ 2>⍞ ∞ 999>⍞"),
                         "3.141592653589793\n1\n1\n1\n")

    def test_numbers(self):
        self.assertEqual(out_of("¯7∣⍞ 7∣⍞ 3 9⊓⍞ 3 9⊔⍞ 0‼⍞ 6‼⍞ 48 36⟌⍞"),
                         "7\n7\n3\n9\n1\n720\n12\n")

    def test_aggregates(self):
        self.assertEqual(out_of("⟨1 2 3 4⟩∑⍞ ⟨⟩∑⍞ ⟨2 3 4⟩∏⍞ ⟨1 2 3 4⟩µ⍞"),
                         "10\n0\n24\n2.5\n")

    def test_list_helpers(self):
        self.assertEqual(
            out_of("⟨7 8 9⟩⊃⍞ ⟨7 8 9⟩⌷⍞ ⟨7 8 9⟩⍫⍞ ⟨7 8 9⟩2⊤⍞ ⟨7 8 9⟩2⊥⍞ ⟨3 1 2⟩⍒⍞"),
            "7\n9\n⟨8 9⟩\n⟨7 8⟩\n⟨9⟩\n⟨3 2 1⟩\n",
        )

    def test_zip(self):
        self.assertEqual(out_of("⟨1 2 3⟩⟨«a» «b»⟩⍚⍞"), "⟨⟨1 «a»⟩ ⟨2 «b»⟩⟩\n")

    def test_text(self):
        self.assertEqual(out_of("«Neo!»⇑⍞ «Neo!»⇩⍞"), "NEO!\nneo!\n")
        self.assertEqual(out_of("« the  matrix »⍭⍞"), "⟨«the» «matrix»⟩\n")
        self.assertEqual(out_of("«a⏎b⏎c»⍖#⍞"), "3\n")

    def test_std_sigils_cannot_be_redefined(self):
        code, _, err = run("1≔π")
        self.assertEqual(code, 1)
        self.assertIn("already defined", err)

    def test_std_available_in_user_boot_and_strands(self):
        self.assertEqual(out_of("π≔c\n⇊\nc τ<⍞"), "1\n")

    def test_std_works_in_spawned_strands(self):
        self.assertEqual(out_of("[⟨4 2⟩⍋↥r]⚡⋈↧r⍞"), "⟨2 4⟩\n")


if __name__ == "__main__":
    unittest.main()
