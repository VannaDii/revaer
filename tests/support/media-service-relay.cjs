const { spawn } = require('node:child_process');
const { createServer } = require('node:http');

function createMediaServiceRelay(app) {
  if (!app) throw new Error('The owned media fixture container is required');
  const children = new Set();
  const server = createServer((req, res) => {
    const args = ['exec', '-i', app, 'curl', '-sS', '--no-buffer', '-D', '-', '-X', req.method,
      'http://127.0.0.1:7070' + req.url];
    for (const [key, value] of Object.entries(req.headers)) {
      if (!['host', 'connection', 'content-length', 'accept-encoding'].includes(key)) {
        args.push('-H', key + ': ' + value);
      }
    }
    if (!['GET', 'HEAD', 'OPTIONS'].includes(req.method)) args.push('--data-binary', '@-');
    const child = spawn('docker', args, { stdio: ['pipe', 'pipe', 'pipe'] });
    children.add(child);
    let pending = Buffer.alloc(0);
    let headersRead = false;
    child.stdout.on('data', chunk => {
      if (headersRead) { res.write(chunk); return; }
      pending = Buffer.concat([pending, chunk]);
      let split = pending.indexOf('\r\n\r\n');
      while (split >= 0) {
        const lines = pending.subarray(0, split).toString().split('\r\n');
        const statusCode = Number(lines[0].split(' ')[1]);
        if (statusCode >= 100 && statusCode < 200) {
          pending = pending.subarray(split + 4);
          split = pending.indexOf('\r\n\r\n');
          continue;
        }
        break;
      }
      if (split < 0) return;
      const lines = pending.subarray(0, split).toString().split('\r\n');
      res.statusCode = Number(lines.shift().split(' ')[1]);
      for (const line of lines) {
        const colon = line.indexOf(':');
        if (colon < 0) continue;
        const key = line.slice(0, colon);
        const value = line.slice(colon + 1).trim();
        if (!['transfer-encoding', 'connection', 'content-length'].includes(key.toLowerCase())) {
          res.setHeader(key, value);
        }
      }
      headersRead = true;
      res.flushHeaders();
      res.write(pending.subarray(split + 4));
    });
    child.on('error', error => {
      console.error('Linux fixture relay failure: ' + error.message);
      if (!res.headersSent) res.statusCode = 502;
      res.end('Linux fixture relay failure');
    });
    child.on('exit', code => {
      children.delete(child);
      if (!headersRead) res.statusCode = 502;
      res.end();
      if (code !== 0 && code !== null) console.error('Linux fixture relay exited ' + code);
    });
    child.stderr.resume();
    req.pipe(child.stdin);
    res.on('close', () => { if (child.exitCode === null) child.kill('SIGTERM'); });
  });
  return {
    server,
    async close() {
      for (const child of children) child.kill('SIGTERM');
      await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
    },
  };
}

module.exports = { createMediaServiceRelay };
