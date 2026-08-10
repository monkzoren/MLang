"""Concurrency (strands, channels, spawn, join), forms, and I/O."""

import os
import subprocess
import sys
import unittest

from helpers import out_of, run

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from mlang.forms import to_flat, to_rain  # noqa: E402

ROOT = os.path.join(os.path.dirname(__file__), "..")


class TestStrands(unittest.TestCase):
    def test_pour_drain(self):
        self.assertEqual(out_of("⟨1 2 3⟩⇈a ⇟a⍞"), "⟨1 2 3⟩\n")
        self.assertEqual(out_of("⟨⟩⇈a ⇟a⍞"), "⟨⟩\n")
        self.assertEqual(out_of("«ab»⇈a ⇟a⍞"), "⟨«a» «b»⟩\n")

    def test_pump_pipeline(self):
        src = "9⍸[1+∂×]∵⇈α\n[2×]⇉αβ\n⇟β[⍞]∀"
        self.assertEqual(out_of(src).split(),
                         [str(2 * n * n) for n in range(1, 10)])

    def test_pump_empty_stream(self):
        self.assertEqual(out_of("⟨⟩⇈a\n[∂×]⇉ab\n⇟b#⍞"), "0\n")

    def test_drain_blocks_until_nil(self):
        # drain in strand 1 must wait for strand 0's pour
        self.assertEqual(out_of("3⍸⇈q\n⇟q⍞"), "⟨0 1 2⟩\n")

    def test_channel_pipeline(self):
        src = ("9⍸[1+∂×↥α]∀∅↥α\n"
               "[↧α∂∅≠][2×↥β]⟳⌫∅↥β\n"
               "[↧β∂∅≠][⍞]⟳⌫\n")
        self.assertEqual(out_of(src).split(),
                         [str(2 * n * n) for n in range(1, 10)])

    def test_channel_fifo_order(self):
        self.assertEqual(out_of("1↥c 2↥c 3↥c\n↧c⍞↧c⍞↧c⍞"), "1\n2\n3\n")

    def test_try_recv(self):
        self.assertEqual(out_of("⇂q⍞ 5↥q ⇂q⍞⍞"), "0\n1\n5\n")

    def test_strand_id_and_count(self):
        self.assertEqual(out_of("⍳↥a\n⍳↥b\n↧a⍞↧b⍞≣⍞"), "0\n1\n3\n")

    def test_spawn_join(self):
        self.assertEqual(out_of("[42↥r]⚡⋈↧r⍞"), "42\n")

    def test_spawn_inherits_locals_by_copy(self):
        self.assertEqual(out_of("7⇒x [x↥r]⚡⋈ 9⇒x ↧r⍞ x⍞"), "7\n9\n")

    def test_join_unknown_strand_glitches(self):
        code, _, err = run("99⋈")
        self.assertEqual(code, 1)
        self.assertIn("no strand", err)

    def test_deadlock_detected(self):
        code, _, err = run("↧a\n↧b")
        self.assertEqual(code, 1)
        self.assertIn("deadlock", err)
        self.assertIn("channel a", err)
        self.assertIn("channel b", err)

    def test_yield_advances(self):
        # regression: ⌛ must not re-execute forever after its slice ends
        self.assertEqual(out_of("2⍸[⌫«A»⍞⌛]∀\n2⍸[⌫«B»⍞⌛]∀"),
                         "A\nB\nA\nB\n")

    def test_deterministic_interleaving(self):
        src = "5⍸[«A»⍞]∀\n5⍸[«B»⍞]∀\n"
        first = out_of(src)
        for _ in range(3):
            self.assertEqual(out_of(src), first)

    def test_boot_runs_before_strands(self):
        src = "[∂×]≔²\n⇊\n7²↥a\n↧a 1+⍞\n"
        self.assertEqual(out_of(src), "50\n")

    def test_boot_glitch_stops_program(self):
        code, out, err = run("1 0÷\n⇊\n«never»⍞")
        self.assertEqual(code, 1)
        self.assertNotIn("never", out)


class TestIO(unittest.TestCase):
    def test_readline_and_eof(self):
        self.assertEqual(out_of("⌨⍞⌨⍞⌨⍞", stdin="a\nb\n"), "a\nb\n∅\n")

    def test_echo_until_eof(self):
        src = "[⌨∂∅≠][⍞]⟳⌫"
        self.assertEqual(out_of(src, stdin="x\ny\n"), "x\ny\n")

    def test_debug_goes_to_stderr(self):
        code, out, err = run("1«s»⟨2 3⟩⍟")
        self.assertEqual(code, 0)
        self.assertEqual(out, "")
        self.assertIn("1 «s» ⟨2 3⟩", err)


class TestForms(unittest.TestCase):
    def test_rain_equals_flat(self):
        flat = ("9⍸[1+∂×↥α]∀∅↥α\n"
                "[↧α∂∅≠][2×↥β]⟳⌫∅↥β\n"
                "[↧β∂∅≠][⍞]⟳⌫\n")
        rain = to_rain(flat)
        self.assertTrue(rain.startswith("⇓"))
        self.assertEqual(out_of(rain), out_of(flat))

    def test_round_trip(self):
        flat = "1 2+⍞\n«hi»⍞\n"
        self.assertEqual(out_of(to_flat(to_rain(flat))), out_of(flat))

    def test_rain_with_boot_divider(self):
        flat = "[∂×]≔²\n⇊\n6²⍞\n"
        rain = to_rain(flat)
        self.assertIn("⇊", rain)
        self.assertEqual(out_of(rain), "36\n")

    def test_continuation_lines(self):
        src = "1 2\n⋮+⍞\n"
        self.assertEqual(out_of(src), "3\n")

    def test_comment_only_lines_are_not_strands(self):
        self.assertEqual(out_of("※ a comment\n⍳⍞\n"), "0\n")

    def test_loose_structural_glyphs_get_clear_errors(self):
        code, _, err = run("⇓«Hello, Matrix»⍞")
        self.assertEqual(code, 2)
        self.assertIn("⇓ marks rain form", err)
        code, _, err = run("1⇊2")
        self.assertEqual(code, 2)
        self.assertIn("boot divider", err)
        code, _, err = run("1 ⋮⍞")
        self.assertEqual(code, 2)
        self.assertIn("⋮ continues a strand", err)
        code, _, err = run("⏎⍞")
        self.assertEqual(code, 2)
        self.assertIn("only meaningful inside", err)

    def test_tabs_rejected(self):
        code, _, err = run("1\t2+⍞")
        self.assertEqual(code, 2)
        self.assertIn("tab", err)

    def test_comment_in_rain_kills_rest_of_column(self):
        rain = "⇓\n1\n⍞\n※\n9\n"
        self.assertEqual(out_of(rain), "1\n")


class TestExamples(unittest.TestCase):
    GOLDEN = {
        "hello.ml": "Hello, Matrix\n",
        "hello-rain.ml": "Hello, Matrix\n",
        "fibonacci.ml": "1 1 2 3 5 8 13 21 34 55 89 144 233 377 610 987",
        "parallel-sum.ml": "500500\n",
        "pipeline.ml": "2 8 18 32 50 72 98 128 162",
        "pipeline-manual.ml": "2 8 18 32 50 72 98 128 162",
        "spawn.ml": ("machine 1 online\nmachine 2 online\n"
                     "machine 3 online\nmachine 4 online\nids sum: 10\n"),
    }

    def _run_file(self, name):
        path = os.path.join(ROOT, "examples", name)
        with open(path, encoding="utf-8") as f:
            return run(f.read())

    def test_examples_golden(self):
        for name, want in self.GOLDEN.items():
            with self.subTest(example=name):
                code, out, err = self._run_file(name)
                self.assertEqual(code, 0, err)
                if "\n" in want:
                    self.assertEqual(out, want)
                else:
                    self.assertEqual(out.split(), want.split())

    def test_fizzbuzz(self):
        code, out, _ = self._run_file("fizzbuzz.ml")
        self.assertEqual(code, 0)
        lines = out.splitlines()
        self.assertEqual(len(lines), 100)
        self.assertEqual(lines[14], "FizzBuzz")
        self.assertEqual(lines[2], "Fizz")
        self.assertEqual(lines[4], "Buzz")
        self.assertEqual(lines[0], "1")

    def test_mandelbrot(self):
        code, out, err = self._run_file("mandelbrot.ml")
        self.assertEqual(code, 0, err)
        lines = out.splitlines()
        self.assertEqual(len(lines), 24)
        self.assertTrue(all(len(l) == 70 for l in lines))
        self.assertIn("█", out)                     # interior of the set
        self.assertEqual(lines[3], lines[21])       # mirror pair around ci=0

    def test_calc(self):
        path = os.path.join(ROOT, "examples", "calc.ml")
        with open(path, encoding="utf-8") as f:
            src = f.read()
        code, out, err = run(src, stdin="3 4 +\n1 0 ÷\noops\n5 5\n")
        self.assertEqual(code, 0, err)
        self.assertEqual(out.splitlines(), [
            "7",
            "✗ ÷ by zero",
            "✗ ⍎ cannot parse «oops» as a number",
            "5",
        ])

    def test_glitch_example_fails_but_survives(self):
        code, out, err = self._run_file("glitch.ml")
        self.assertEqual(code, 1)
        self.assertIn("strand 0 alive", out)
        self.assertIn("strand 1 alive", out)
        self.assertIn("out of bounds", err)

    def test_cli_runs(self):
        r = subprocess.run(
            [sys.executable, "-m", "mlang", "eval", "«cli»⍞"],
            capture_output=True, text=True, cwd=ROOT, timeout=30,
        )
        self.assertEqual(r.returncode, 0)
        self.assertEqual(r.stdout, "cli\n")


if __name__ == "__main__":
    unittest.main()
