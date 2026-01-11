# Epic 2 Coordination Guide

**Epic**: Storage Layer
**Branch**: `epic-2-storage-layer`
**Stories**: #12, #13, #14, #15, #16

---

## Quick Start

### 1. Create Epic Branch
```bash
cd /Users/aniruddhajoshi/Documents/GitHub/in-mem
git checkout develop
git pull origin develop
git checkout -b epic-2-storage-layer
git push -u origin epic-2-storage-layer
```

### 2. Assign Stories to Claudes

| Story | Component | Claude | Dependency | Estimated | Can Start |
|-------|-----------|--------|------------|-----------|-----------|
| #12 | UnifiedStore | Claude 1 | None (start immediately) | 5-6 hours | ✅ Now |
| #13 | Secondary indices | Claude 2 | Blocked by #12 | 4-5 hours | ⏳ After #12 |
| #14 | TTL index | Claude 3 | Blocked by #12 | 4-5 hours | ⏳ After #12 |
| #15 | Snapshot view | Claude 4 | Blocked by #12 | 3-4 hours | ⏳ After #12 |
| #16 | Storage tests | Claude 5 | Blocked by #13,#14,#15 | 3-4 hours | ⏳ After all |

---

## Phase 1: Foundation (Now)

**Start Story #12 immediately**:

1. Open new Claude conversation
2. Copy prompt from `github-issues/epic-2-claude-prompts.md` → Prompt 1
3. Claude implements UnifiedStore
4. Claude creates PR to `epic-2-storage-layer`
5. Review and merge PR

**Expected**: 5-6 hours for Story #12 completion

---

## Phase 2: Parallel Development (After #12 Merges)

Once Story #12 PR merges to `epic-2-storage-layer`, **start 3 Claudes in parallel**:

### Claude 2: Story #13 (Secondary Indices)
- Copy Prompt 2 from `epic-2-claude-prompts.md`
- Works on `crates/storage/src/index.rs`
- Modifies `unified.rs` to use indices

### Claude 3: Story #14 (TTL Index)
- Copy Prompt 3 from `epic-2-claude-prompts.md`
- Works on `crates/storage/src/ttl.rs` and `cleaner.rs`
- Modifies `unified.rs` to use TTL index

### Claude 4: Story #15 (Snapshot View)
- Copy Prompt 4 from `epic-2-claude-prompts.md`
- Works on `crates/storage/src/snapshot.rs`
- Adds `create_snapshot()` to `unified.rs`

**File Conflict Management**:
- All 3 will modify `unified.rs` - expect merge conflicts
- Story #13 and #15 are safest to run first (minimal overlap)
- Story #14 can merge last if conflicts arise

**Expected**: 4-5 hours wall time (parallel)

---

## Phase 3: Integration Testing (After #13, #14, #15 Merge)

Once all 3 stories (#13, #14, #15) merge:

### Claude 5: Story #16 (Comprehensive Tests)
- Copy Prompt 5 from `epic-2-claude-prompts.md`
- Creates `crates/storage/tests/integration_tests.rs`
- Creates `crates/storage/tests/stress_tests.rs`
- Runs coverage report (target: ≥85%)

**Expected**: 3-4 hours

---

## Total Timeline

**Sequential Execution**: 19-24 hours

**Parallel Execution** (recommended):
- Phase 1: 5-6 hours (Story #12)
- Phase 2: 4-5 hours (Stories #13, #14, #15 in parallel)
- Phase 3: 3-4 hours (Story #16)
- **Total**: 12-15 hours wall time

**Speedup**: ~2x with parallelization

---

## Communication Protocol

### When Starting Work
Comment on your GitHub issue:
```
Starting work on Story #X. Branch: epic-2-story-X-<name>
ETA: <hours>
```

### When Blocked
Comment on your GitHub issue:
```
Blocked waiting for Story #Y to merge. Will start when ready.
```

### When Creating PR
Comment on your GitHub issue:
```
PR created: #<PR_NUMBER>
Ready for review. Notifying downstream dependencies.
```

Then comment on dependent issues:
```
Story #X merged to epic-2-storage-layer. You can now start.
```

### When Merged
Close your issue or add comment:
```
✅ Merged to epic-2-storage-layer
```

---

## Merge Conflict Resolution

If multiple PRs modify `unified.rs`:

### Strategy 1: Sequential Merging
1. Merge #13 first (secondary indices)
2. Merge #15 second (snapshot) - rebase on #13
3. Merge #14 last (TTL) - rebase on #13+#15

### Strategy 2: Coordinate via GitHub
- Claude 2, 3, 4 coordinate in issue comments
- Agree on who rebases when conflicts occur
- Use `scripts/sync-epic.sh 2` to pull latest epic branch

### Emergency: Manual Conflict Resolution
If Claudes can't resolve conflicts:
```bash
git checkout epic-2-storage-layer
git pull origin epic-2-storage-layer

# Manually resolve unified.rs conflicts
# Then push

git push origin epic-2-storage-layer
```

---

## Quality Gates

### Per-Story Quality Gate
Each story must pass before merging:
- [ ] All tests pass: `cargo test -p in-mem-storage`
- [ ] No clippy warnings: `cargo clippy -p in-mem-storage -- -D warnings`
- [ ] Code formatted: `cargo fmt -p in-mem-storage`
- [ ] PR description complete
- [ ] Acceptance criteria met

### Epic 2 Quality Gate (After All Stories)
Before merging epic to develop:
- [ ] Run epic review: `./scripts/review-epic.sh 2`
- [ ] Test coverage ≥85% for storage layer
- [ ] All 5 PRs merged to epic branch
- [ ] Epic-specific tests pass (Story #16)
- [ ] No regressions in core layer
- [ ] Fill out `docs/milestones/EPIC_2_REVIEW.md`

---

## File Ownership Map

To minimize conflicts, here's which Claude owns which files:

### Claude 1 (Story #12) - Creates Foundation
- `crates/storage/src/unified.rs` - **CREATES**
- `crates/storage/Cargo.toml` - **CREATES**
- `crates/storage/src/lib.rs` - **CREATES**

### Claude 2 (Story #13) - Adds Indices
- `crates/storage/src/index.rs` - **CREATES**
- `crates/storage/src/unified.rs` - **MODIFIES** (adds index fields, updates put/delete)
- `crates/storage/src/lib.rs` - **MODIFIES** (exports index module)

### Claude 3 (Story #14) - Adds TTL
- `crates/storage/src/ttl.rs` - **CREATES**
- `crates/storage/src/cleaner.rs` - **CREATES**
- `crates/storage/src/unified.rs` - **MODIFIES** (adds ttl_index field, updates put/delete)
- `crates/storage/src/lib.rs` - **MODIFIES** (exports ttl, cleaner modules)

### Claude 4 (Story #15) - Adds Snapshots
- `crates/storage/src/snapshot.rs` - **CREATES**
- `crates/storage/src/unified.rs` - **MODIFIES** (adds create_snapshot method)
- `crates/storage/src/lib.rs` - **MODIFIES** (exports snapshot module)

### Claude 5 (Story #16) - Adds Tests
- `crates/storage/tests/integration_tests.rs` - **CREATES**
- `crates/storage/tests/stress_tests.rs` - **CREATES**
- No modifications to existing files (only reads)

**Conflict Risk**: `unified.rs` and `lib.rs` are modified by stories #13, #14, #15
**Mitigation**: Sequential merging (#13 → #15 → #14) or coordination

---

## Progress Tracking

### Story Status

| Story | Status | PR | Merged | Blocker |
|-------|--------|-----|--------|---------|
| #12 | ⏳ Not started | - | ❌ | None |
| #13 | ⏸️ Blocked | - | ❌ | #12 |
| #14 | ⏸️ Blocked | - | ❌ | #12 |
| #15 | ⏸️ Blocked | - | ❌ | #12 |
| #16 | ⏸️ Blocked | - | ❌ | #13,#14,#15 |

Update this table as stories progress.

---

## Troubleshooting

### Problem: cargo not found in Claude
**Solution**: Scripts auto-source `~/.cargo/env`. If Claude still has issues, tell Claude to run:
```bash
source "$HOME/.cargo/env"
cargo --version
```

### Problem: Merge conflict in unified.rs
**Solution**: Use `scripts/sync-epic.sh 2` to pull latest, then manually resolve:
```bash
# Pull latest epic branch
git checkout epic-2-story-X-name
git fetch origin epic-2-storage-layer
git rebase origin/epic-2-storage-layer

# Resolve conflicts in unified.rs
# Then continue
git rebase --continue
git push --force-with-lease
```

### Problem: Tests fail after merging another story
**Solution**: Rebase on latest epic branch and re-run tests:
```bash
git checkout epic-2-story-X-name
git fetch origin epic-2-storage-layer
git rebase origin/epic-2-storage-layer
cargo test -p in-mem-storage
```

### Problem: Story taking longer than estimated
**Solution**: Comment on GitHub issue with status update. Others can help review code or debug.

---

## After Epic 2 Complete

1. **Run Epic Review**:
   ```bash
   ./scripts/review-epic.sh 2
   ```

2. **Fill Out Review**:
   - Edit `docs/milestones/EPIC_2_REVIEW.md`
   - Mark all checklists
   - Add notes on any issues

3. **Merge to Develop** (if approved):
   ```bash
   git checkout develop
   git merge epic-2-storage-layer --no-ff
   git push origin develop
   ```

4. **Tag Release**:
   ```bash
   git tag epic-2-complete -m "Epic 2: Storage Layer Complete"
   git push origin epic-2-complete
   ```

5. **Close Epic Issue**:
   ```bash
   gh issue close 2 --comment "Epic 2 complete. All 5 stories merged. Review: docs/milestones/EPIC_2_REVIEW.md"
   ```

6. **Update Project Status**:
   - Edit `docs/milestones/PROJECT_STATUS.md`
   - Mark Epic 2 complete
   - Update progress: 11/27 stories (40%), 2/5 epics (40%)

7. **Create Epic Summary**:
   - Create `docs/milestones/EPIC_2_SUMMARY.md`
   - Document what was built, metrics, lessons learned

8. **Begin Epic 3**: WAL Implementation

---

## Ready to Start?

1. Create epic branch (see Quick Start above)
2. Open new Claude conversation
3. Copy Prompt 1 from `github-issues/epic-2-claude-prompts.md`
4. Let Claude implement Story #12

**Good luck!** 🚀
