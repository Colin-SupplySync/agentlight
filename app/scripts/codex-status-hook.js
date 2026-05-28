import http from 'node:http';

const chunks = [];

const exitSuccessfully = () => process.exit(0);

try {
  process.stdin.on('data', (chunk) => {
    chunks.push(chunk);
  });

  process.stdin.on('error', exitSuccessfully);

  process.stdin.on('end', () => {
    try {
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
          response.on('end', exitSuccessfully);
          response.on('error', exitSuccessfully);
        }
      );

      request.on('error', exitSuccessfully);
      request.on('timeout', () => {
        request.destroy();
        exitSuccessfully();
      });

      request.end(body);
    } catch {
      exitSuccessfully();
    }
  });

  process.stdin.resume();
} catch {
  exitSuccessfully();
}
