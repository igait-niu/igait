# GitHub Project Board

iGait's canonical board is **project #2** under the `igait-niu` org. A project
#1 exists but is stale/unused — issues aren't routed there.

If `gh project item-edit` ever returns *"The item does not exist in the project,"*
you almost certainly targeted project #1 by mistake. Re-resolve IDs against #2.

## Quick ID reference (cached from `gh project field-list`)

| Thing | ID |
|-------|-----|
| Project #2 | `PVT_kwDOChRl_c4BT9BW` |
| Status field | `PVTSSF_lADOChRl_c4BT9BWzhBIaMc` |
| Priority field | `PVTSSF_lADOChRl_c4BT9BWzhBIaQk` |
| Size field | `PVTSSF_lADOChRl_c4BT9BWzhBIaQo` |

### Status options

| Name | ID |
|------|-----|
| Planning | `f75ad846` |
| Ready | `61e4505c` |
| In progress | `47fc9ee4` |
| In review | `df73e18b` |
| Done | `98236657` |

### Priority options

| Name | ID |
|------|-----|
| ASAP | `79628723` |
| Soon | `0a877460` |
| Long-Term | `da944a9c` |

> ⚠️ There is **no** "Later" option — if someone says "Later," they probably
> mean Soon or Long-Term. Ask.

### Size options

| Name | ID |
|------|-----|
| Miniscule | `6c6483d2` |
| Small | `f784b110` |
| Moderate | `7515a9f1` |
| Large | `817d0097` |
| Gigantic | `db339eb2` |

## Recipes

### Add an issue to the board

```bash
gh project item-add 2 --owner igait-niu \
  --url https://github.com/igait-niu/igait/issues/<N> --format json | jq -r .id
```

That ID is what you pass to subsequent `item-edit` calls.

### Set Priority + Size on a newly-added item

```bash
ITEM_ID=<from step above>
gh project item-edit --id "$ITEM_ID" \
  --project-id PVT_kwDOChRl_c4BT9BW \
  --field-id PVTSSF_lADOChRl_c4BT9BWzhBIaQk \
  --single-select-option-id 0a877460   # Soon
gh project item-edit --id "$ITEM_ID" \
  --project-id PVT_kwDOChRl_c4BT9BW \
  --field-id PVTSSF_lADOChRl_c4BT9BWzhBIaQo \
  --single-select-option-id f784b110   # Small
```

### Move an item's Status

```bash
gh project item-edit --id "$ITEM_ID" \
  --project-id PVT_kwDOChRl_c4BT9BW \
  --field-id PVTSSF_lADOChRl_c4BT9BWzhBIaMc \
  --single-select-option-id df73e18b   # In review
```

### Find an item ID by issue number

```bash
gh project item-list 2 --owner igait-niu --format json --limit 200 \
  | jq '.items[] | select(.content.number==<N>) | {id, title: .content.title, status}'
```

## Refreshing these IDs

Project/field IDs rarely change, but option IDs can if someone renames a
column. If anything here goes stale:

```bash
gh project field-list 2 --owner igait-niu --format json \
  | jq '.fields[] | select(.name=="Status" or .name=="Priority" or .name=="Size")'
```
