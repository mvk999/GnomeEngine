#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

node --input-type=module - "$@" <<'NODE'
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve('assets/wallpapers');
const stageRoot = process.argv[2] ? path.resolve(process.argv[2]) : null;
const strict = process.env.GNOMEENGINE_REQUIRE_STARTER_ASSETS === '1';
const requiredFiles = [];
const ids = new Set();
let complete = 0;

const fail = (where, message) => {
    throw new Error(`${where}: ${message}`);
};
const relativeAsset = (directory, relative, field) => {
    if (typeof relative !== 'string' || relative.length === 0 || path.isAbsolute(relative)) {
        fail(directory, `${field} must be a non-empty relative path`);
    }
    if (relative.split(/[\\/]/).includes('..')) {
        fail(directory, `${field} must not contain parent-directory components`);
    }
    const resolved = path.resolve(directory, relative);
    const relation = path.relative(directory, resolved);
    if (relation === '..' || relation.startsWith(`..${path.sep}`) || path.isAbsolute(relation)) {
        fail(directory, `${field} escapes the wallpaper directory`);
    }
    return resolved;
};
const assertRegularContained = (directory, file, field) => {
    let current = directory;
    for (const part of path.relative(directory, file).split(path.sep)) {
        current = path.join(current, part);
        const stat = fs.lstatSync(current);
        if (stat.isSymbolicLink()) fail(directory, `${field} must not traverse symbolic links`);
    }
    const stat = fs.lstatSync(file);
    if (!stat.isFile()) fail(directory, `${field} must resolve to a regular file`);
    const canonicalRoot = fs.realpathSync(directory);
    const canonicalFile = fs.realpathSync(file);
    const relation = path.relative(canonicalRoot, canonicalFile);
    if (relation === '..' || relation.startsWith(`..${path.sep}`) || path.isAbsolute(relation)) {
        fail(directory, `${field} escapes the wallpaper directory`);
    }
};

if (stageRoot) fs.mkdirSync(stageRoot, { recursive: true, mode: 0o755 });
if (!fs.existsSync(root)) {
    if (strict) fail(root, 'no built-in wallpaper assets are present');
    console.log('No built-in wallpaper assets yet; development/package build may continue.');
    process.exit(0);
}

for (const name of fs.readdirSync(root).sort()) {
    const directory = path.join(root, name);
    const directoryStat = fs.lstatSync(directory);
    if (directoryStat.isSymbolicLink() || !directoryStat.isDirectory()) continue;
    const manifestPath = path.join(directory, 'manifest.json');
    if (!fs.existsSync(manifestPath)) continue;
    const manifestStat = fs.lstatSync(manifestPath);
    if (manifestStat.isSymbolicLink() || !manifestStat.isFile() || manifestStat.size > 1024 * 1024) {
        fail(name, 'manifest must be a regular file no larger than 1 MiB');
    }
    let manifest;
    try {
        manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    } catch (error) {
        fail(name, `manifest JSON is invalid: ${error.message}`);
    }
    if (manifest.schemaVersion !== 1 || typeof manifest.id !== 'string'
        || !/^[A-Za-z0-9-]{8,64}$/.test(manifest.id)
        || typeof manifest.title !== 'string' || manifest.title.trim() === ''
        || manifest.type !== 'video' || typeof manifest.createdAt !== 'string'
        || typeof manifest.content?.entry !== 'string'
        || manifest.media === null || typeof manifest.media !== 'object' || Array.isArray(manifest.media)) {
        fail(name, 'manifest does not conform to the supported version-1 video schema');
    }
    if (ids.has(manifest.id)) fail(name, `duplicate wallpaper ID ${manifest.id}`);
    ids.add(manifest.id);
    const media = relativeAsset(directory, manifest.content.entry, 'content.entry');
    const thumbnail = relativeAsset(directory, manifest.thumbnail ?? 'thumbnail.png', 'thumbnail');
    const mediaExists = fs.existsSync(media);
    const thumbnailExists = fs.existsSync(thumbnail);
    if (!mediaExists || !thumbnailExists) {
        if (strict) fail(name, 'manifest, referenced video, and referenced thumbnail must all be present');
        console.log(`Skipping incomplete built-in wallpaper: ${name}`);
        continue;
    }
    for (const [file, field] of [[media, 'content.entry'], [thumbnail, 'thumbnail']]) {
        assertRegularContained(directory, file, field);
    }
    if (strict && ['author', 'copyright', 'license'].some(key => typeof manifest[key] !== 'string' || !manifest[key].trim())) {
        fail(name, 'release assets must declare author, copyright, and license');
    }
    requiredFiles.push({ name, manifestPath, media, thumbnail });
    complete++;
}

if (strict && complete === 0) fail(root, 'no complete built-in wallpaper is ready for release');
if (stageRoot) {
    for (const entry of requiredFiles) {
        const destination = path.join(stageRoot, entry.name);
        fs.mkdirSync(destination, { recursive: true, mode: 0o755 });
        for (const source of [entry.manifestPath, entry.thumbnail, entry.media]) {
            const relative = path.relative(path.join(root, entry.name), source);
            const target = path.join(destination, relative);
            fs.mkdirSync(path.dirname(target), { recursive: true, mode: 0o755 });
            fs.copyFileSync(source, target);
            fs.chmodSync(target, 0o644);
        }
    }
}
console.log(`Validated ${complete} complete built-in wallpaper(s).`);
NODE
