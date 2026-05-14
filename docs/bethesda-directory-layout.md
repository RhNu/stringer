# Bethesda Directory Layout Notes

Bethesda Gamebryo and Creation Engine mods usually model files as if they live
under the game `Data` directory. The same logical paths may be installed as loose
files or packed into BSA/BA2 archives. For Stringer v1 reader work, loose files
with the same logical path should override archive entries.

## Text Assets Relevant To Stringer

Common paths that can contain text or localization data:

- `Data/*.esm`, `Data/*.esp`, `Data/*.esl`: plugin data files.
- `Data/Strings/*_<Language>.STRINGS`: normal localized strings.
- `Data/Strings/*_<Language>.DLSTRINGS`: dialogue-style localized strings.
- `Data/Strings/*_<Language>.ILSTRINGS`: indexed localized strings.
- `Data/Scripts/*.pex`: compiled Papyrus bytecode. Skyrim-era Papyrus scripts
  are external files instead of embedded directly in plugin records.
- `Data/Interface/Translations/*_<Language>.txt`: Scaleform translation tables.

## Common Non-V1 Asset Paths

Archives and loose folders often also contain non-text resources that the first
reader crate intentionally ignores:

- `Data/Meshes`
- `Data/Textures`
- `Data/Sound`
- `Data/Interface/*.swf`
- `Data/Seq`
- `Data/SKSE`

These paths matter to gameplay and packaging, but they are not loaded into
`FileBundle` until Stringer needs non-text asset discovery.
