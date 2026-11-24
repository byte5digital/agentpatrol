# Evaluations

The AI Judgement eval of specification section 4.4, one engine at a time. Every
figure here comes from a report of the eval harness (`judgement-eval`, run as
hand check 25 describes), taken on the author's own Mac. The reports themselves
are not committed. Nothing of the corpora is recorded here beyond the set
sizes: no content, no path and no name of an Item.

Add a dated section for every engine run rather than editing an old one. The
table in the core that the daemon's engine answer reads holds one row per
measured engine, model and `prompt_version`, within the bar or outside it, and
follows these sections. A prompt change raises `prompt_version`, which leaves
every engine not measured until it is run again.

## What makes a result count

- **The shipping bar** is the only gated figure: at most 2 % of the benign
  Items in the Ambiguous Band called malicious, after the quote check, over at
  least thirty of them. The rate over every benign Item stands beside it for
  information. An engine that misses the bar, or cannot be measured against
  it, ships disabled in the Simple Posture.
- **Recall** on the bad set counts once the bad corpus holds at least three
  samples for each class T3 can see and thirty in all. The bad Heuristic
  Fixtures are asked too, but they do not make the bad set sufficient.
- **The injection flip-rate** needs the wrapper list, which wraps each bad Item
  in reviewer-directed text. Recall and flip-rate are reported, never gated.

## Why no engine can pass yet

The benign corpus holds 29 Items in the Ambiguous Band, one short of the thirty
the bar is measured over, so every engine reads "cannot pass" whatever its
rate. The bad corpus holds no sample yet, so recall is over the bad Heuristic
Fixtures alone and is insufficient in every class. The wrapper list is empty,
so no injection variant was built and no flip-rate was measured. Apple and
Ollama are outside the bar and ship disabled in the Simple Posture. Claude
called no benign Item malicious, and the owner accepted it as within the bar
on 2026-11-24 despite the 29 Items.

## 2026-11-24, Apple on-device, macOS 26.6.2 (25G83), prompt version 2

The model is the full macOS version and build the Apple judgement helper
reported; its row in the table names the major version, 26, and holds for
every release of it.

Sets: 130 benign Items, 29 of them in the Ambiguous Band. No bad corpus
sample; of the 125 bad Heuristic Fixtures, the 119 of Threat Heuristics in a
class T3 can see make the bad set, 87 of them in the Ambiguous Band. No
injection variant.

| Figure | Measured | |
|---|---|---|
| False malicious in the Ambiguous Band (gated) | 6 of 29, 20.69 % | outside, and too few Items |
| False malicious over every benign Item | 27 of 130, 20.77 % | for information |
| Recall over every bad Item | 63 of 119, 52.94 % | insufficient bad set |
| Recall in the Ambiguous Band | 46 of 87, 52.87 % | insufficient bad set |
| Injection flip-rate | not measured | no wrapper |

No Verdict: 58 benign Items (20 engine errors, 5 too large, 33 failing the
quote check) and 47 bad Items (all failing the quote check). The benign
Verdicts were 27 malicious, 44 benign and 1 unsure.

## 2026-11-24, Ollama, gemma3:27b, prompt version 2

The model is the tag with the digest Ollama reported,
`gemma3:27b@a418f5838eaf7fe2cfe0a3046c8384b68ba43a4435542c942f9db00a5f342203`;
its row in the table names the tag alone. It is the one Ollama model measured.

Sets: as for Apple above.

| Figure | Measured | |
|---|---|---|
| False malicious in the Ambiguous Band (gated) | 17 of 29, 58.62 % | outside, and too few Items |
| False malicious over every benign Item | 42 of 130, 32.31 % | for information |
| Recall over every bad Item | 104 of 119, 87.39 % | insufficient bad set |
| Recall in the Ambiguous Band | 78 of 87, 89.66 % | insufficient bad set |
| Injection flip-rate | not measured | no wrapper |

No Verdict: 22 benign Items (2 too large, 20 failing the quote check) and 6
bad Items (all failing the quote check). The benign Verdicts were 42
malicious, 60 benign and 6 unsure.

The token estimate: Ollama counted the prompt of 247 answered requests, and
one of them counted above the estimate of three bytes a token the request was
built under. The highest count was 113 % of its estimate.

## 2026-11-24, Claude API, claude-sonnet-5, prompt version 2

Sets: as for Apple above.

| Figure | Measured | |
|---|---|---|
| False malicious in the Ambiguous Band (gated) | 0 of 29, 0.00 % | too few Items to count |
| False malicious over every benign Item | 0 of 130, 0.00 % | for information |
| Recall over every bad Item | 60 of 119, 50.42 % | insufficient bad set |
| Recall in the Ambiguous Band | 49 of 87, 56.32 % | insufficient bad set |
| Injection flip-rate | not measured | no wrapper |

No Verdict: no benign Item and 10 bad Items (1 engine error, 9 failing the
quote check). The benign Verdicts were 114 benign and 16 unsure. The rate is
under the bar. Over 29 Items it does not count by the rule above; the owner
accepted Claude as within the bar on 2026-11-24 all the same.
