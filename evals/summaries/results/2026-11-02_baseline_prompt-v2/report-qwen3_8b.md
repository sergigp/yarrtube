# Summary eval: prompt v2, judge qwen3:8b

| model | contradicted | critical stated | facts stated | code checks | avg time |
|---|---|---|---|---|---|
| qwen3:4b | 3/50 | 15/22 | 26/50 | 35/40 | 328s |
| gemma3:4b | 1/50 | 16/22 | 27/50 | 36/40 | 92s |

## Failed critical facts and contradictions

- **qwen3:4b** · accelerated-dragon · `game1-technical-endgame-win` → contradicted: contradicted: The summary states the first game resulted in a loss (minus 4), while the fact claims the streamer won.
- **qwen3:4b** · accelerated-dragon · `game2-advantage-lost-then-won` → contradicted: contradicted: The summary says the second game ended in a draw, but the fact claims the streamer still won after opponent mistakes.
- **qwen3:4b** · accelerated-dragon · `game3-draw-missed-win` → missing: missing: The summary does not mention the streamer missing a win via a knight f4 double attack, only that the game ended in a draw.
- **qwen3:4b** · argentine-anarchism · `1880-1916-is-oligarchy-narrative` → missing: missing: The summary does not address the claim about 1880–1916 being a 'golden age' under the conservative order or its narrative.
- **qwen3:4b** · browser-tabs · `built-browser-tabs-cli` → contradicted: The summary states the tool was developed by Codely, not the speaker, contradicting the claim that the speaker built it.
- **qwen3:4b** · build-claude-code · `topic-own-harness` → missing: The summary states the creator built a harness in Go but does not mention using the Anthropic API, which is part of the fact's main claim.
- **qwen3:4b** · extreme-places · `five-places-covered` → missing: missing
- **qwen3:4b** · extreme-places · `neom-fallen-apart` → missing: missing
- **qwen3:4b** · future-of-capitalism · `decoupling-would-come-too-late` → missing: missing
- **gemma3:4b** · accelerated-dragon · `three-games-as-black` → missing: missing
- **gemma3:4b** · argentine-anarchism · `anarchist-golden-age-early-1900s` → missing: missing
- **gemma3:4b** · argentine-anarchism · `1880-1916-is-oligarchy-narrative` → contradicted: contradicted
- **gemma3:4b** · extreme-places · `neom-fallen-apart` → missing: missing: The summary does not mention the creator's afterword about NEOM falling apart due to physical and budget constraints.
- **gemma3:4b** · future-of-capitalism · `decoupling-would-come-too-late` → missing: missing
- **gemma3:4b** · quantum-ai · `advantage-only-on-artificial-problem` → missing: missing: The summary mentions examples like data classification but does not specify the roulette squares example or its artificial nature, so the fact's claim about the example is not addressed.

## Judge vs human verdicts: 0/4 agree

- qwen3:4b · accelerated-dragon · `game3-draw-missed-win`: human stated, judge missing
- qwen3:4b · browser-tabs · `built-browser-tabs-cli`: human stated, judge contradicted
- qwen3:4b · browser-tabs · `open-source-semantic-release`: human stated, judge missing
- qwen3:4b · browser-tabs · `worth-it-saves-time`: human stated, judge missing

## Code checks failed

- gemma3:4b · browser-tabs: direct_start
- qwen3:4b · browser-tabs: direct_start
- gemma3:4b · build-claude-code: direct_start
- qwen3:4b · cross-entropy: direct_start
- qwen3:4b · extreme-places: length_3_to_5_sentences, direct_start
- gemma3:4b · future-of-capitalism: direct_start
- gemma3:4b · quantum-ai: direct_start
- qwen3:4b · stonewall-e4-knight: direct_start
