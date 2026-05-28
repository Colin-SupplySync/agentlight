const http = require('http');

const chunks = [];

process.stdin.on('data', (chunk) => {
  chunks.push(chunk);
});

process.stdin.on('end', () => {
  const body = Buffer.concat(chunks);
  const request = http.request(
    {
      hostname: '127.0.0.1',
      port: 17321,
      path: '/codex-hook',
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'content-length': body.length,
      },
      timeout: 400,
    },
    (response) => {
      response.resume();
      response.on('end', () => process.exit(0));
    }
  );

  request.on('error', () => process.exit(0));
  request.on('timeout', () => {
    request.destroy();
    process.exit(0);
  });

  request.end(body);
});

process.stdin.resume();
