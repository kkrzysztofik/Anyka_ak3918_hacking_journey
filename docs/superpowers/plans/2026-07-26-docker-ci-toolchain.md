# Docker CI Image Toolchain Upgrade Plan

> **For agentic workers:** Execute on `integrate-video` (no feature branch).

**Goal:** Publish `kkrzysztofik/anyka-cross-compile:rust-1.97.1` (+ `:latest`) from the local 1.97.1 toolchain and retarget main CI.

**Spec:** `docs/superpowers/specs/2026-07-26-docker-ci-toolchain-design.md`

## File map

| File | Role |
|------|------|
| `scripts/docker/Dockerfile` | Drop rustup stable; keep tarpaulin/Snyk; PATH hygiene |
| `scripts/docker/docker-build.sh` | Build entry (tag via `-t`) |
| `scripts/docker/test-docker-image.sh` | Smoke tests; update push hint for versioned tag |
| `.github/workflows/main-ci.yml` | Image → `rust-1.97.1` |
| `.serena/memories/project-context.md` | Container tag |
| `.serena/memories/suggested_commands.md` | Container tag |

---

### Task 1: Dockerfile hygiene

- [ ] Remove rustup `stable` install block (vendored cargo is real ELF).
- [ ] Install `cargo-tarpaulin` via `/opt/.../bin/cargo`.
- [ ] Keep PATH with `/opt/arm-anykav200-crosstool-ng/bin` first; avoid putting rustup bin ahead.
- [ ] Assert `rustc` path + version contains `1.97`.

### Task 2: Workflow + docs

- [ ] `main-ci.yml`: `kkrzysztofik/anyka-cross-compile:rust-1.97.1`
- [ ] Update Serena memories mentioning `rust-1.91.1`
- [ ] Extend `test-docker-image.sh` push hints for `rust-1.97.1` + `latest`

### Task 3: Build, test, push

```bash
./scripts/docker/docker-build.sh --tag anyka-cross-compile:rust-1.97.1 --no-cache
IMAGE_TAG=anyka-cross-compile:rust-1.97.1 ./scripts/docker/test-docker-image.sh
docker tag anyka-cross-compile:rust-1.97.1 kkrzysztofik/anyka-cross-compile:rust-1.97.1
docker tag anyka-cross-compile:rust-1.97.1 kkrzysztofik/anyka-cross-compile:latest
docker push kkrzysztofik/anyka-cross-compile:rust-1.97.1
docker push kkrzysztofik/anyka-cross-compile:latest
```

### Task 4: Commit + push integrate-video

- [ ] Commit Dockerfile/workflow/docs/memories/spec/plan
- [ ] `git push origin integrate-video`
