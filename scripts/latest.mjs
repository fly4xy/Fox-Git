// After `tauri build`: copies the release files into dist-windows/ or dist-linux/ and writes
// the update manifests next to them, which the app reads from the server (src-tauri/src/update.rs).
// Upload the files first and the latest-*.json after them, or the app may fetch a missing file.
//
//   node scripts/latest.mjs windows   ->  Fox-Git_<v>_x64-setup.exe, fox-git.exe, latest-setup.json, latest-portable.json
//   node scripts/latest.mjs linux     ->  Fox-Git_<v>_amd64.deb, latest-deb.json
import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const BASE = 'https://downloads.lis-carter.site/fox-git/';
const root = path.resolve(import.meta.dirname, '..');
const release = path.join(root, 'src-tauri', 'target', 'release');
const { version } = JSON.parse(readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));

/** The bundle tauri made for this version, whatever spaces its product name has. */
function bundle(dir, suffix) {
	const found = readdirSync(path.join(release, 'bundle', dir)).find((name) => name.endsWith(suffix));
	if (!found) throw new Error(`No *${suffix} in bundle/${dir}: build version ${version} first`);
	return path.join(release, 'bundle', dir, found);
}

function publish(dist, source, file, manifest, query = '') {
	mkdirSync(dist, { recursive: true });
	const target = path.join(dist, file);
	copyFileSync(source, target);
	const data = readFileSync(target);
	const entry = {
		version,
		file,
		url: BASE + file + query,
		size: statSync(target).size,
		sha256: createHash('sha256').update(data).digest('hex')
	};
	writeFileSync(path.join(dist, manifest), JSON.stringify(entry) + '\n');
	console.log(`${manifest}: ${file} ${entry.size} bytes`);
}

const target = process.argv[2];
if (target === 'windows') {
	const dist = path.join(root, 'dist-windows');
	publish(dist, bundle('nsis', `_${version}_x64-setup.exe`), `Fox-Git_${version}_x64-setup.exe`, 'latest-setup.json');
	// The portable exe keeps one name, so the query keeps caches from serving an old copy.
	publish(dist, path.join(release, 'fox-git.exe'), 'fox-git.exe', 'latest-portable.json', `?v=${version}`);
} else if (target === 'linux') {
	publish(path.join(root, 'dist-linux'), bundle('deb', `_${version}_amd64.deb`), `Fox-Git_${version}_amd64.deb`, 'latest-deb.json');
} else {
	console.error('Usage: node scripts/latest.mjs windows|linux');
	process.exit(1);
}
