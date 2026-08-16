'use strict';

const assert = require('node:assert/strict');
const config = require('../../release/release.config.js');

assert.ok(Array.isArray(config.branches));
assert.ok(Array.isArray(config.plugins));
assert.equal(typeof config.tagFormat, 'string');
