#### Features
- c9d340c67aa2e60b31f30bdc371c842d2b797ee9 - show ~/.norsedata cache size on splash screen - @stephenfairchild, Claude Sonnet 4.6

- 780fd3c92e9e555b04c1ef2668b3c93745ca51a0 - cache PR summaries in ~/.norsedata/repos/ and use as LLM context - @stephenfairchild, Claude Sonnet 4.6

- 61a4ac7422f68d7bd7dd4ade13babfda0540a24c - always require curl example for any API change - @stephenfairchild, Claude Sonnet 4.6

- 1a1a9d143902900d481be495d7cdbdfebb6c39d1 - require full API details in summary when diff contains API changes - @stephenfairchild, Claude Sonnet 4.6

- a69028820172b1927026dd48dc4934da069d157b - add ## Usage section with code example to AI diff summary - @stephenfairchild, Claude Sonnet 4.6

- 86298007a35956ddf53e0dc588866ade30033a21 - show PR comments below AI summary in diff view - @stephenfairchild, Claude Sonnet 4.6

- 030a3eed5c9ca14dd0700cfa4b74817ec85096a8 - press R in diff view to post a PR comment - @stephenfairchild, Claude Sonnet 4.6

- 55902f48144d9588266a995f56bd44ba6651f460 - add PR metadata status bar above AI summary in diff view - @stephenfairchild, Claude Sonnet 4.6

- 4148b4185c8ec1742f551242fb3f3c55dc86c4f5 - persist approved PRs to ~/.norsedata/prs-approved - @stephenfairchild, Claude Sonnet 4.6

- 225f6de48b4210f106050a839568cc4dcbdb4f9a - add PR approval, news activity, recently closed, and Jira extraction - @stephenfairchild, Claude Sonnet 4.6

- 98819ca71c73aa23787c4645df1e83d16b3a1449 - add model picker with persistence via ~/.norsedata/model - @stephenfairchild, Claude Sonnet 4.6

- 8a6be7ec3cdb0abffb02d59fcd0d8d23e460817f - read config from ~/.norse instead of config.toml - @stephenfairchild, Claude Sonnet 4.6

- d8abe418e0dab46a8b6e9252f5054ef99cc44702 - make org configurable - @stephenfairchild

#### Bug Fixes
- eccde7467c10eb1c5da01fefc32deede5006a223 - show all timestamps as relative time, not raw dates - @stephenfairchild, Claude Sonnet 4.6

- 4ffe7f2aa8308cad059d7dc900f9394900511e77 - fetch all PR comment types, not just issue-level discussion - @stephenfairchild, Claude Sonnet 4.6

- 6be1665e8c127c5c8b5092fb4f29c7256f50dd11 - load live approval state from GitHub when opening a PR diff - @stephenfairchild, Claude Sonnet 4.6

- de62545fb85cd835ebe84ea537dcff2ce2b5b7b2 - rename acv-terminal to norse in user-agent strings - @stephenfairchild, Claude Sonnet 4.6

#### Documentation
- b234255861909b568540951c74193cf4f44404cf - fix repo owner in install URLs - @stephenfairchild, Claude Sonnet 4.6

- 5c772f5e243073d4737c3a7a570582cfeda6e186 - add install instructions and fix test config path - @stephenfairchild, Claude Sonnet 4.6

- 6232c5493c05e9942a151e15fcaea3bcddc2a6be - add README - @stephenfairchild, Claude Sonnet 4.6

- 7dbbd41453db44016e46446ed1033ee9d48a3bb1 - add features and keybindings reference - @stephenfairchild, Claude Sonnet 4.6

#### Tests
- a4071902cb6c6fa9ddddb3e531fe190a8f69cb55 - add a test harness - @stephenfairchild

#### Continuous Integration
- d504806e70b75458518354d05347d8e0c35d87f0 - replace release-please with cocogitto PR accumulation workflow - @stephenfairchild, Claude Sonnet 4.6

- a5f35ccee022a650f06433c208b6f014ecdeb99d - replace release-please with cocogitto - @stephenfairchild, Claude Sonnet 4.6

- 8351d9a9230fcc0a3a9827164a32fa81b5b4464b - reset Cargo.toml to 0.0.0 for release-please to manage - @stephenfairchild, Claude Sonnet 4.6

- c279cb7ae879cf11816b486e20c3db1effe1674a - restore manifest at 0.0.0 with v0.0.0 tag as anchor - @stephenfairchild, Claude Sonnet 4.6

- 8569947404b91ac49dbdcaecbbe07149618d45aa - remove manifest so release-please recreates it from scratch - @stephenfairchild, Claude Sonnet 4.6

- d17492689e9710c1e28abdee6591ce020a841367 - remove bootstrap-sha - @stephenfairchild, Claude Sonnet 4.6

- e3c48b568a7ce1fcc8f9073701cb95e6c79686d4 - use full bootstrap-sha - @stephenfairchild, Claude Sonnet 4.6

- f8713a9f9018b9b70cf30329a609c3828929ebc1 - add bootstrap-sha so release-please scans from initial commit - @stephenfairchild, Claude Sonnet 4.6

- 5fa49deb5b0f1fd74d8751f2c1ab9ae8ba54a438 - add workflow_dispatch trigger to release-please - @stephenfairchild, Claude Sonnet 4.6

- 498a8abd2b815cfba0f0287e9d74b4c28db1925f - add binary release workflow for all platforms - @stephenfairchild, Claude Sonnet 4.6

- cdffcefe9d9762a21da872762ff16b16c9a450b6 - add release-please config with custom PR header - @stephenfairchild, Claude Sonnet 4.6

- b0ae6beeb1bbc70b0d9d449f16868165ae70ea87 - add GitHub Actions workflows and commitlint - @stephenfairchild, Claude Sonnet 4.6

#### Miscellaneous Chores
- 48fcd5c44d43add4c5228282eaccadef9581a836 - release v0.1.0 - @stephenfairchild

- 3895f23307e8b656e56fe1b820e0c32ddde857d2 - release v0.1.0 - stephenfairchild

- f1a03de11af09cee73ca6b7d4711adf34e542118 - trigger release-please - @stephenfairchild, Claude Sonnet 4.6

- 4116eacbd7cf452737de91b02b91126afc37c707 - reset manifest to 0.0.0 so release-please picks up all commits - @stephenfairchild, Claude Sonnet 4.6

- 738cd396902ed4ac392314bcaa840a60c7a464dd - add release-please manifest - @stephenfairchild, Claude Sonnet 4.6



