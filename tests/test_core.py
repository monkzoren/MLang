"""Literals, stack ops, arithmetic, comparison, logic."""

import unittest

from helpers import out_of, run


class TestLiterals(unittest.TestCase):
    def test_integers_and_runs(self):
        self.assertEqual(out_of("42⍞"), "42\n")
        self.assertEqual(out_of("1 2 3++⍞"), "6\n")

    def test_negative_and_float(self):
        self.assertEqual(out_of("¯5⍞"), "¯5\n")
        self.assertEqual(out_of("2.5⍞"), "2.5\n")
        self.assertEqual(out_of(".5 2×⍞"), "1.0\n")

    def test_bignum(self):
        self.assertEqual(out_of("2 100^⍞").strip(), str(2**100))

    def test_string_with_newline_glyph(self):
        self.assertEqual(out_of("«a⏎b»⊸"), "a\nb")

    def test_list_literal(self):
        self.assertEqual(out_of("⟨1 «two» ⟨3⟩⟩⍞"), "⟨1 «two» ⟨3⟩⟩\n")

    def test_nil(self):
        self.assertEqual(out_of("∅⍞ ∅∅=⍞ ∅0=⍞"), "∅\n1\n0\n")

    def test_lone_negative_sign_is_load_error(self):
        code, _, err = run("¯⍞")
        self.assertEqual(code, 2)
        self.assertIn("lone ¯", err)

    def test_unterminated_string(self):
        code, _, err = run("«abc")
        self.assertEqual(code, 2)
        self.assertIn("unterminated", err)

    def test_unclosed_quotation(self):
        code, _, err = run("[1 2")
        self.assertEqual(code, 2)
        self.assertIn("unclosed", err)

    def test_double_dot_number(self):
        code, _, err = run("1.2.3⍞")
        self.assertEqual(code, 2)


class TestStack(unittest.TestCase):
    def test_shuffles(self):
        self.assertEqual(out_of("1∂+⍞"), "2\n")                # dup
        self.assertEqual(out_of("1 2⇅-⍞"), "1\n")              # swap
        self.assertEqual(out_of("1 2⌫⍞"), "1\n")               # drop
        self.assertEqual(out_of("1 2⊚⍞⍞⍞"), "1\n2\n1\n")       # over
        self.assertEqual(out_of("1 2 3⥀⍞⍞⍞"), "1\n3\n2\n")     # rot
        self.assertEqual(out_of("7 8≢⍞"), "2\n")               # depth

    def test_underflow_glitches(self):
        code, _, err = run("+")
        self.assertEqual(code, 1)
        self.assertIn("underflow", err)


class TestArithmetic(unittest.TestCase):
    def test_basics(self):
        self.assertEqual(out_of("3 4+⍞ 10 3-⍞ 6 7×⍞"), "7\n7\n42\n")

    def test_division_stays_int_when_exact(self):
        self.assertEqual(out_of("8 2÷⍞ 7 2÷⍞"), "4\n3.5\n")

    def test_div_and_mod_by_zero_glitch(self):
        code, _, err = run("1 0÷")
        self.assertEqual(code, 1)
        self.assertIn("÷ by zero", err)
        code, _, err = run("1 0%")
        self.assertEqual(code, 1)

    def test_unary(self):
        self.assertEqual(out_of("2√⍞ 2.7⌊⍞ 2.1⌈⍞ 5±⍞"),
                         "1.4142135623730951\n2\n3\n¯5\n")

    def test_sqrt_negative_glitches(self):
        code, _, err = run("¯1√")
        self.assertEqual(code, 1)

    def test_type_error_glitches(self):
        code, _, err = run("1«x»+")
        self.assertEqual(code, 1)
        self.assertIn("expects numbers", err)


class TestCompareLogic(unittest.TestCase):
    def test_comparisons(self):
        self.assertEqual(out_of("1 2<⍞ 2 2≤⍞ 3 2>⍞ 1 2≥⍞ 1 1=⍞ 1 2≠⍞"),
                         "1\n1\n1\n0\n1\n1\n")

    def test_string_compare(self):
        self.assertEqual(out_of("«a»«b»<⍞"), "1\n")

    def test_mixed_compare_glitches(self):
        code, _, _ = run("1«a»<")
        self.assertEqual(code, 1)

    def test_logic(self):
        self.assertEqual(out_of("1 0∧⍞ 1 0∨⍞ 0¬⍞ 1 1⊻⍞"), "0\n1\n1\n0\n")


if __name__ == "__main__":
    unittest.main()
