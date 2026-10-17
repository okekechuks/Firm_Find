"""Read-only normal-data evidence. Never migrate, checkpoint, repair or reset SQLite."""
import ctypes
import hashlib
import json
from pathlib import Path
import sqlite3
import sys

buffer = ctypes.create_unicode_buffer(32768)
if ctypes.windll.shell32.SHGetFolderPathW(None, 28, None, 0, buffer) != 0:
    raise SystemExit('Windows LocalAppData is unavailable')
root = Path(buffer.value) / 'OBTAINER'
database = root / 'obtainer.db'
def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()
def database_files():
    return {p.name: {'bytes': p.stat().st_size, 'sha256': digest(p)} for p in [database, Path(str(database)+'-wal')] if p.is_file()}
files_before = database_files()
with sqlite3.connect(database.as_uri()+'?mode=ro', uri=True) as c:
    c.execute('PRAGMA query_only=ON')
    tables = [r[0] for r in c.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
    rows = {t: sorted(c.execute('SELECT * FROM "'+t.replace('"','""')+'"').fetchall(), key=repr) for t in tables}
    report = {'data_root': str(root), 'vault_root': str(root/'vault'), 'schema': c.execute('PRAGMA user_version').fetchone()[0],
              'integrity': c.execute('PRAGMA integrity_check').fetchall(), 'foreign_keys': c.execute('PRAGMA foreign_key_check').fetchall(),
              'table_counts': {t: len(r) for t,r in rows.items()}, 'all_rows_sha256': hashlib.sha256(repr(rows).encode()).hexdigest(),
              'settings_sha256': hashlib.sha256(json.dumps(rows['settings']).encode()).hexdigest(),
              'saved_router_ids': [r[0] for r in rows['manual_routers']], 'profile_ids': [r[0] for r in rows['profile_versions']]}
report['database_files'] = database_files()
assert files_before == report['database_files'], 'Database changed during read-only probe; normal application may be active'
blobs = {}
for p in (root/'vault').rglob('*'):
    if p.is_symlink() or p.stat().st_file_attributes & 0x400:
        raise SystemExit('Unexpected normal-vault reparse point; no traversal permitted')
    if p.is_file() and p.name != 'library.lock':
        blobs[str(p.relative_to(root/'vault'))] = {'bytes': p.stat().st_size, 'sha256': digest(p)}
report['vault_files'] = blobs
report['other_directories'] = {}
for name in ('logs', 'workspaces', 'reports'):
    files = {}
    for p in (root/name).rglob('*'):
        if p.is_symlink() or p.stat().st_file_attributes & 0x400:
            raise SystemExit('Unexpected normal-data reparse point; probe stopped')
        if p.is_file():
            files[str(p.relative_to(root/name))] = {'bytes': p.stat().st_size, 'sha256': digest(p)}
    report['other_directories'][name] = files
assert report['integrity'] == [('ok',)] and not report['foreign_keys']
encoded = json.dumps(report, indent=2)+'\n'
Path(sys.argv[1]).write_text(encoded, encoding='utf-8')
if len(sys.argv)>2:
    before = json.loads(Path(sys.argv[2]).read_text(encoding='utf-8'))
    assert before == json.loads(encoded), 'Normal application data changed during isolated tests'
print(encoded)
