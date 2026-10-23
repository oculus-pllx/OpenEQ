# Parallax OpenEQ

This fork starts from daeken/OpenEQ commit
`9c0e5fc210b26fb2a32f06a0686b53749adfbcc6`. `main` preserves RoF2 while
Parallax maintains tested protocol and headless tooling.

`upstream` is review-only; imports are pinned and tested. EverQuest assets,
Daybreak binaries, private captures, credentials, and MacroQuest are excluded.

## ToB boundary

ToB is not implemented at this baseline. Future support uses explicit
client-family protocol descriptions and evidence from the owner's lawful client
plus public EQEmu/Parallax server definitions. Unsupported families fail closed.

## Release licensing hold

The workspace declares `MIT OR Apache-2.0` in Cargo metadata, but the baseline
contains no top-level license texts. Preserve attribution and do not publish
binary releases until that discrepancy is resolved with upstream provenance.
