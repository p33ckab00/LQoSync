"""Move Trunk-generated module code to a file for the strict script policy."""
from pathlib import Path
import re
root = Path(__file__).resolve().parents[2] / 'crates/web-ui/dist'
index = root / 'index.html'
html = index.read_text()
pattern = r'<script\b(?![^>]*\bsrc=)[^>]*type="module"[^>]*>(.*?)</script>'
modules = re.findall(pattern, html, re.S)
if len(modules) != 1:
    raise SystemExit('Expected one generated Trunk module. Check the build output.')
(root / 'bootstrap.js').write_text(modules[0])
index.write_text(re.sub(pattern, '<script type="module" src="/assets/bootstrap.js"></script>', html, flags=re.S))

# Serve deterministic gzip variants when the browser accepts gzip.
import gzip
for asset in root.iterdir():
    if asset.suffix in {".js", ".css", ".wasm", ".ttf", ".svg"}:
        asset.with_name(asset.name + ".gz").write_bytes(gzip.compress(asset.read_bytes(), mtime=0))

# Keep the font license with the deployed font.
from shutil import copyfile
copyfile(root.parent / "FONT-LICENSE.txt", root / "FONT-LICENSE.txt")
