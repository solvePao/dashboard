const apiUrl = 'https://dashboard-api.solvepao.com';
const sites = ['https://homehub.solvepao.com', 'https://nest.solvepao.com'];

class DeploymentVerifier {
  async request(url, options = {}) {
    // Allow time for a newly attached domain's certificate and routing to activate.
    for (let attempt = 1; attempt <= 12; attempt++) {
      try {
        const response = await fetch(url, {
          ...options,
          signal: AbortSignal.timeout(15_000),
          redirect: 'error',
        });
        if (response.status < 500 || attempt === 12) return response;
      } catch (error) {
        if (attempt === 12) throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, 5_000));
    }
  }

  require(condition, message) {
    if (!condition) throw new Error(message);
  }

  async verifyApi() {
    const auth = await this.request(`${apiUrl}/api/auth/me`);
    this.require(auth.status === 401, `API authentication expected 401, got ${auth.status}`);
    for (const origin of [...sites, 'https://unrelated.example']) {
      const response = await this.request(`${apiUrl}/api/auth/google`, {
        method: 'OPTIONS',
        headers: {
          Origin: origin,
          'Access-Control-Request-Method': 'POST',
          'Access-Control-Request-Headers': 'content-type,authorization',
        },
      });
      const expected = sites.includes(origin) ? origin : null;
      this.require(response.status === 204, `CORS preflight failed for ${origin}`);
      this.require(response.headers.get('access-control-allow-origin') === expected,
        `Unexpected CORS allow-origin for ${origin}`);
    }
    console.log(`Verified API authentication and CORS: ${apiUrl}`);
  }

  async verifySites() {
    for (const site of sites) {
      for (const path of ['/', '/deployment-spa-check']) {
        const response = await this.request(`${site}${path}`);
        this.require(response.status === 200, `${site}${path} returned ${response.status}`);
        this.require(response.headers.get('content-type')?.includes('text/html'),
          `${site}${path} did not serve HTML`);
        this.require((await response.text()).includes('id="app"'), `${site}${path} is missing the Vue app`);
      }
      console.log(`Verified HTTPS and SPA routing: ${site}`);
    }
  }
}

const verifier = new DeploymentVerifier();
await verifier.verifyApi();
if (!process.argv.includes('--api-only')) await verifier.verifySites();
