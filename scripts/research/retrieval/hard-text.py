"""Freeze primary-source Hard Text IR documents without storing article text in Git."""

import concurrent.futures, hashlib, json, re, urllib.request
from pathlib import Path
from bs4 import BeautifulSoup
root = Path('data/research/corpus/hard-text')
root.mkdir(exist_ok=True, parents=True)
manifest_path = Path('docs/research/corpus/hard-text.json')
frozen = {s['id']: s['raw_sha256'] for s in json.loads(manifest_path.read_text())['sources']} if manifest_path.exists() else {}
old = json.loads(Path('docs/research/corpus/personal.json').read_text())
sources = [{'id': s['id'], 'url': s['url'], 'cluster': 'personal_chronology', 'entity_ref': s['entity_ref'], 'publication_date': s['publication_date'], 'rights': s['rights']} for s in old['sources']]
sources += [{'id': sid, 'url': url, 'cluster': 'personal_chronology', 'entity_ref': 'entity:research:person:simon-willison', 'publication_date': date, 'rights': 'Publisher copyright; article text retained only in ignored research cache.'} for sid, url, date in [('simon-productivity', 'https://simonwillison.net/2022/Nov/26/productivity/', '2022-11-26'), ('simon-datasette-lite', 'https://simonwillison.net/2022/May/4/datasette-lite/', '2022-05-04'), ('simon-url-python', 'https://simonwillison.net/2025/Feb/13/url-addressable-python/', '2025-02-13'), ('simon-year-llms', 'https://simonwillison.net/2025/Dec/31/the-year-in-llms/', '2025-12-31')]]
for v in range(5, 15):
    sources.append({'id': f'python-3-{v}', 'url': f'https://docs.python.org/3.{v}/whatsnew/3.{v}.html', 'cluster': 'evolving_python', 'entity_ref': 'entity:research:project:python', 'revision': f'3.{v}', 'rights': 'Python documentation PSF License; third-party text retained only in ignored research cache.'})
for v in ['9.6'] + [str(n) for n in range(10, 19)]:
    rv = v if v == '9.6' else v + '.0'
    sources.append({'id': 'postgresql-' + v.replace('.', '-'), 'url': f'https://www.postgresql.org/docs/release/{rv}/', 'cluster': 'database_revision_confounders', 'entity_ref': 'entity:research:project:postgresql', 'revision': v, 'rights': 'PostgreSQL documentation PostgreSQL License; third-party text retained only in ignored research cache.'})

def acquire(s):
    p = root / (s['id'] + '.html')
    if not p.exists():
        req = urllib.request.Request(s['url'], headers={'User-Agent': 'Nous-Wave research corpus source freezer'})
        with urllib.request.urlopen(req, timeout=60) as f:
            raw = f.read()
            s['resolved_url'] = f.url
        p.write_bytes(raw)
    else:
        raw = p.read_bytes()
        s['resolved_url'] = s['url']
    if s['id'] in frozen and hashlib.sha256(raw).hexdigest() != frozen[s['id']]:
        raise ValueError('Frozen source digest changed: ' + s['id'])
    soup = BeautifulSoup(raw, 'html.parser')
    if s['cluster'] == 'personal_chronology':
        main = soup.select_one('.entry.entryPage')
    elif s['cluster'] == 'evolving_python':
        main = soup.select_one('div.body') or soup.select_one('main')
    else:
        main = soup.select_one('#docContent') or soup.select_one('.sect1')
    if main is None:
        raise ValueError('Missing article body ' + s['url'])
    for e in main.select('script,style,.entryFooter,.toc,.sphinxsidebar,nav'):
        e.decompose()
    s['topic'] = (main.find('h1') or main.find('h2') or soup.find('title')).get_text(' ', strip=True)
    heading = []
    units = []
    for e in main.find_all(['h1', 'h2', 'h3', 'h4', 'p', 'li', 'pre']):
        if e.name in ['p', 'li', 'pre'] and e.find_parent(['p', 'li', 'pre']):
            continue
        text = re.sub('\\s+', ' ', e.get_text(' ', strip=True)).strip()
        if e.name.startswith('h'):
            level = int(e.name[1])
            heading = heading[:level - 1]
            heading.append(text.rstrip('¶'))
            continue
        if len(text) < 80 or not re.search('[A-Za-z]', text):
            continue
        anchor = e.get('id')
        if not anchor:
            ancestor = e.find_parent(id=True)
            anchor = ancestor.get('id') if ancestor else None
        uid = f"{s['id']}-u{len(units) + 1:04}"
        units.append({'id': uid, 'source': s['id'], 'ordinal': len(units), 'heading_path': heading.copy(), 'anchor': anchor, 'text': text, 'sha256': hashlib.sha256(text.encode()).hexdigest()})
    (root / (s['id'] + '.units.json')).write_text(json.dumps(units, ensure_ascii=False, indent=2) + '\n')
    s['raw_sha256'] = hashlib.sha256(raw).hexdigest()
    s['unit_count'] = len(units)
    s['extraction'] = 'article-body-leaf-blocks-v1: p/li/pre >=80 chars; nested duplicate blocks excluded; whitespace normalized; heading path and nearest HTML anchor retained'
    print(s['id'], len(units), flush=True)
    return (s, units)
results = []
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    for result in pool.map(acquire, sources):
        results.append(result)
manifest = {'version': 1, 'status': 'sources_frozen_queries_pending_audit', 'sources': [s for s, u in results], 'units': [{k: v for k, v in unit.items() if k not in ('text', 'heading_path')} for s, u in results for unit in u]}
manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n')
(root / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n')
(root / 'unit-texts.json').write_text(json.dumps({u['id']: u['text'] for s, units in results for u in units}, ensure_ascii=False) + '\n')
print('TOTAL', len(manifest['sources']), len(manifest['units']), flush=True)
