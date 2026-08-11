"""Quotations, control flow, iteration, sequences, bindings."""

import unittest

from helpers import out_of, run


class TestControl(unittest.TestCase):
    def test_apply(self):
        self.assertEqual(out_of("[1 2+]!⍞"), "3\n")

    def test_if_with_quotations(self):
        self.assertEqual(out_of("1[«y»][«n»]?⍞ 0[«y»][«n»]?⍞"), "y\nn\n")

    def test_if_with_values(self):
        self.assertEqual(out_of("1 5 9?⍞"), "5\n")

    def test_while_fibonacci(self):
        out = out_of("0 1[∂100<][∂⍞⇅⊚+]⟳⌫⌫")
        self.assertEqual(out.split(), "1 1 2 3 5 8 13 21 34 55 89".split())

    def test_repeat(self):
        self.assertEqual(out_of("0 5[1+]⍣⍞"), "5\n")

    def test_nested_quotations(self):
        self.assertEqual(out_of("[[3]!]!⍞"), "3\n")


class TestIteration(unittest.TestCase):
    def test_range_map(self):
        self.assertEqual(out_of("5⍸[∂×]∵⍞"), "⟨0 1 4 9 16⟩\n")

    def test_each(self):
        self.assertEqual(out_of("3⍸[⍞]∀"), "0\n1\n2\n")

    def test_filter(self):
        self.assertEqual(out_of("10⍸[2%0=]⌿⍞"), "⟨0 2 4 6 8⟩\n")

    def test_fold(self):
        self.assertEqual(out_of("101⍸ 0[+]⍀⍞"), "5050\n")

    def test_iterate_string(self):
        self.assertEqual(out_of("«abc»[⍞]∀"), "a\nb\nc\n")

    def test_map_over_empty(self):
        self.assertEqual(out_of("0⍸[∂×]∵⍞"), "⟨⟩\n")


class TestSequences(unittest.TestCase):
    def test_length_concat_index(self):
        self.assertEqual(out_of("«abc»#⍞ ⟨1 2⟩⟨3⟩⧺⍞ «ab»«cd»⧺⍞ ⟨7 8 9⟩1@⍞"),
                         "3\n⟨1 2 3⟩\nabcd\n8\n")

    def test_index_out_of_bounds(self):
        code, _, err = run("⟨1⟩5@")
        self.assertEqual(code, 1)
        self.assertIn("out of bounds", err)

    def test_slice_clamps(self):
        self.assertEqual(out_of("«matrix»1 4⊂⍞ «ab»0 99⊂⍞"), "atr\nab\n")

    def test_split_join(self):
        self.assertEqual(out_of("«a,b,c»«,»⊆⍞"), "⟨«a» «b» «c»⟩\n")
        self.assertEqual(out_of("«ab»«»⊆⍞"), "⟨«a» «b»⟩\n")
        self.assertEqual(out_of("⟨1 2 3⟩«-»⊇⍞"), "1-2-3\n")

    def test_str_parse(self):
        self.assertEqual(out_of("42⍕«!»⧺⍞ «¯3»⍎⍞ «2.5»⍎⍞"), "42!\n¯3\n2.5\n")

    def test_parse_garbage_glitches(self):
        code, _, _ = run("«nope»⍎")
        self.assertEqual(code, 1)

    def test_codepoints(self):
        self.assertEqual(out_of("«A»⌗⍞ 66⍘⍞"), "65\nB\n")


class TestBindings(unittest.TestCase):
    def test_define_and_call(self):
        self.assertEqual(out_of("[∂×]≔² 9²⍞"), "81\n")

    def test_define_constant(self):
        self.assertEqual(out_of("3.14159≔κ κ⍞"), "3.14159\n")

    def test_redefinition_glitches(self):
        code, _, err = run("1≔x 2≔x")
        self.assertEqual(code, 1)
        self.assertIn("already defined", err)

    def test_reserved_sigil_is_load_error(self):
        code, _, err = run("1≔+")
        self.assertEqual(code, 2)
        self.assertIn("reserved", err)

    def test_locals_rebind_freely(self):
        self.assertEqual(out_of("1⇒x x 1+⇒x x⍞"), "2\n")

    def test_undefined_sigil_glitches(self):
        code, _, err = run("Ω")
        self.assertEqual(code, 1)
        self.assertIn("undefined sigil", err)

    def test_locals_do_not_cross_strands(self):
        # strand 0 defines local x; strand 1 must not see it
        code, out, err = run("1⇒x 0↥s\n↧s⌫x⍞")
        self.assertEqual(code, 1)
        self.assertIn("undefined sigil 'x'", err)


class TestGlitches(unittest.TestCase):
    def test_try_catches(self):
        self.assertEqual(out_of("[1 0÷][«got: »⇅⍕⧺⍞]⍥«after»⍞"),
                         "got: ÷ by zero\nafter\n")

    def test_try_restores_stack_depth(self):
        # body pushes garbage before glitching; handler sees a clean stack
        self.assertEqual(out_of("7[1 2 3 0÷][⌫]⍥⍞"), "7\n")

    def test_raise_custom_value(self):
        self.assertEqual(out_of("[⟨1 2⟩↯][⍞]⍥"), "⟨1 2⟩\n")

    def test_nested_try(self):
        self.assertEqual(out_of("[[«inner»↯][«re-»⇅⍕⧺↯]⍥][⍞]⍥"), "re-inner\n")

    def test_uncaught_kills_only_its_strand(self):
        code, out, err = run("1 0÷\n«survivor»⍞")
        self.assertEqual(code, 1)
        self.assertIn("survivor", out)
        self.assertIn("÷ by zero", err)

    def test_success_disarms_try(self):
        self.assertEqual(out_of("[1][«no»⍞]⍥⍞"), "1\n")


if __name__ == "__main__":
    unittest.main()
