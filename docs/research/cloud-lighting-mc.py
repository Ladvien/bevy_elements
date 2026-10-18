"""Monte Carlo reference for a homogeneous, conservative (albedo 1) cloud slab
(docs/research/cloud-lighting.md; the table is src/clouds/monte_carlo_slab.in).

Phase: the march's dual-lobe Henyey-Greenstein 0.7 HG(0.8) + 0.3 HG(-0.5). A sun of unit
perpendicular illuminance from zenith cosine mu0 lights the slab's top; photons are followed
until they leave, and those that leave after at least one scattering are binned by direction
(the unscattered beam is the sun seen through the cloud, which the composite draws). Prints
the rows of the table, L/E in sr^-1, for the optical depths on the command line:

    uv run --no-project --with numpy python cloud-lighting-mc.py 1 3 10 40
"""
import sys
import numpy as np

rng = np.random.default_rng(7)


def sample_hg(g, u):
    s = (1 - g * g) / (1 - g + 2 * g * u)
    return (1 + g * g - s * s) / (2 * g)


def scatter(d, n):
    lobe = rng.random(n) < 0.7
    g = np.where(lobe, 0.8, -0.5)
    cos_t = sample_hg(g, rng.random(n))
    sin_t = np.sqrt(np.maximum(0.0, 1 - cos_t * cos_t))
    phi = 2 * np.pi * rng.random(n)
    # orthonormal basis around d
    a = np.where(np.abs(d[:, 2:3]) < 0.9, np.array([[0, 0, 1.0]]), np.array([[1.0, 0, 0]]))
    u = np.cross(a, d)
    u /= np.linalg.norm(u, axis=1, keepdims=True)
    v = np.cross(d, u)
    return d * cos_t[:, None] + u * (sin_t * np.cos(phi))[:, None] + v * (sin_t * np.sin(phi))[:, None]


def run(tau, mu0, n=2_000_000, max_events=4000):
    # z up, slab from z = 0 (base) to z = tau (optical units). Light comes from above.
    s0 = np.sqrt(1 - mu0 * mu0)
    d = np.tile(np.array([-s0, 0.0, -mu0]), (n, 1))  # travelling direction (down, away from sun)
    z = np.full(n, tau)
    alive = np.ones(n, bool)
    out_top = []
    out_bot = []
    first = True
    for _ in range(max_events):
        idx = np.nonzero(alive)[0]
        if idx.size == 0:
            break
        dd = d[idx]
        step = -np.log(rng.random(idx.size))
        zn = z[idx] + dd[:, 2] * step
        top = zn > tau
        bot = zn < 0
        # The unscattered beam is the sun seen through the cloud, drawn separately.
        if top.any() and not first:
            out_top.append(dd[top])
        if bot.any() and not first:
            out_bot.append(dd[bot])
        first = False
        esc = top | bot
        alive[idx[esc]] = False
        keep = ~esc
        k = idx[keep]
        z[k] = zn[keep]
        d[k] = scatter(dd[keep], k.size)
    return np.concatenate(out_top or [np.zeros((0, 3))]), np.concatenate(out_bot or [np.zeros((0, 3))]), n


def radiance(dirs, n, mu0, up, mu_c, az_c, dmu=0.1, daz=np.radians(30)):
    """L/E toward the viewer in the escape direction (zenith cosine mu_c, azimuth az_c in
    degrees from the sun's azimuth seen from the viewer side)."""
    mu = dirs[:, 2] if up else -dirs[:, 2]
    # The sun shines from +x toward -x, so an escape direction toward +x leaves on the sun's
    # side: a viewer there has the sun behind them.
    az = np.degrees(np.arctan2(dirs[:, 1], dirs[:, 0]))
    dist = np.abs(((az - az_c + 180) % 360) - 180)
    sel = (np.abs(mu - mu_c) < dmu / 2) & (dist < np.degrees(daz) / 2)
    omega = dmu * daz
    return sel.sum() / n * mu0 / (mu_c * omega)


if __name__ == "__main__":
    for tau in [float(t) for t in sys.argv[1:]] or [1.0, 3.0, 10.0, 40.0]:
        for mu0 in [0.9, 0.5]:
            top, bot, n = run(tau, mu0)
            rows = []
            for mu_c in [0.95, 0.7, 0.4]:
                for az in [0.0, 90.0, 180.0]:
                    rows.append((tau, mu0, 0, mu_c, az, radiance(top, n, mu0, True, mu_c, az)))
                rows.append((tau, mu0, 1, mu_c, 180.0, radiance(bot, n, mu0, False, mu_c, 180.0)))
            for r in rows:
                print(f"    ({r[0]:.1f}, {r[1]:.2f}, {r[2]}, {r[3]:.2f}, {r[4]:.1f}, {r[5]:.6f}),")
