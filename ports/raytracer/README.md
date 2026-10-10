# raytracer: Ray Tracing in One Weekend, in kanso

A port of the renderer from Peter Shirley's *Ray Tracing in One Weekend*
(with Trevor David Black and Steve Hollasch, version 4,
<https://raytracing.github.io/books/RayTracingInOneWeekend.html>). The book
builds a path tracer in about a thousand lines of C++ over fourteen
chapters: vectors and rays, spheres, antialiasing, diffuse, metal and glass
materials, a positionable camera with defocus blur, and a final scene of a
few hundred random spheres. This port was written from the book's
explanations, not from its source.

## What it does

    kanso run . -- <scene | file.scene> [--width N] [--samples N]
                   [--depth N] [--seed N] [--grid N] [--shade S]

writes a PPM (P3) image to stdout and the original's "Scanlines remaining"
progress to stderr.

| scene     | the book's chapter                                          |
|-----------|-------------------------------------------------------------|
| `sky`     | 4: the background gradient alone                            |
| `diffuse` | 10: a Lambertian sphere on a Lambertian ground              |
| `metal`   | 10: matte between fuzzy and sharp metal                     |
| `glass`   | 11: matte, glass with an air bubble, fuzzy gold             |
| `wide`    | 12: the same spheres from (-2, 2, 1) through a 20° lens      |
| `defocus` | 13: the same view with a 10° aperture focused at 3.4        |
| `cover`   | 14: the book's cover, a grid of random spheres              |

`--shade` picks one of the book's shading models: `materials` (the default
and the book's final model), `normals` (chapter 6, colour by surface
normal) or `hemisphere` (chapter 9, uniform bounce off every surface,
ignoring materials).

`--grid N` sets the half-width of the cover's grid of small spheres. The
book uses 11 (up to 484 spheres); smaller grids are for quick renders.

A scene can also be read from a file whose name ends in `.scene`:

    # a comment
    camera aspect 16 9          # width and height of the image's ratio
    camera lookfrom 13 2 3
    camera lookat 0 0 0
    camera vup 0 1 0
    camera vfov 20              # vertical field of view, degrees
    camera defocus 0.6          # aperture angle, degrees; 0 is a pinhole
    camera focus 10             # distance to the plane in focus
    material ground lambertian 0.5 0.5 0.5
    material gold metal 0.8 0.6 0.2 0.1      # albedo, then fuzz
    material glass dielectric 1.5            # refractive index
    sphere 0 -1000 0 1000 ground             # centre, radius, material

Every camera setting is optional and defaults to the camera of chapters 4
to 11. A material must be defined before a sphere uses it. A line the
parser cannot read stops the run with its line number. `scenes/three.scene`
is chapter 11's scene, and `check.sh` confirms its image is byte for byte
the built-in `glass` scene's.

To look at an image, any PPM viewer works; `convert out.ppm out.png` with
ImageMagick is one way.

## What is the same and what differs

The algorithms follow the book: the sphere intersection with the half-b
form of the quadratic, front-face normals, Lambertian scattering as normal
plus random unit vector with the near-zero guard, fuzzy reflection with
absorption below the surface, refraction with total internal reflection and
Schlick's approximation, the camera frame from lookfrom/lookat/vup, the
defocus disk, gamma 2, and `int(256 * clamp(x, 0, 0.999))` per channel.

Differences:

- **Random numbers.** The original calls a global C++ generator. This port
  uses Park and Miller's minimal standard generator (`rng/`), seeded by
  `--seed`, and threads it through every function that draws, so a seed
  fixes the image and all three kanso engines produce the same bytes. The
  cover scene's layout and the render draw from one stream, in the order
  the original draws them.
- **Trigonometry.** kanso's standard library has no `tan`, so `fmath/`
  computes it from Taylor series. The camera only needs angles up to 90
  degrees, where the series is exact to double precision.
- **The near-zero rejection threshold** in `random_unit_vector` is 1e-10
  rather than the book's 1e-160, because kanso has no exponent notation and
  1e-160 written out does not fit in 80 columns.
- **Defaults** are chapter 13's: 400 pixels wide, 100 samples, 50 bounces.
  The book's final 1200×675 at 500 samples is far too slow here; see
  FRICTION.md, F21.

Left out: the book's intermediate code stages that the shading models above
do not cover (the red sphere of chapter 5, the non-gamma-corrected output),
and anything from the later books (BVH, textures, lights).

## Layout

    main.kso         entry: hands the command line to cli
    cli/             argument parsing and error messages
    tracer/          the renderer, one namespace across files:
      ray.kso          rays
      interval.kso     parameter intervals
      sphere.kso       spheres, groups of objects, hit records
      material.kso     lambertian, metal, dielectric, and `scatter`
      camera.kso       camera setup and ray generation
      render.kso       ray colour, sampling, PPM output, shading models
      scenes.kso       the book's scenes
      build.kso        constructors for other modules
    scenefile/       the .scene format
    vec/             three-component vectors and their operators
    rng/             the seeded generator and the random vectors built on it
    fmath/           abs, floor, clamp, sin, cos, tan and friends
    scenes/          example scene files
    fixtures/        <name>.args, and the expected .stdout and .stderr
    bugs/            minimal reproductions of compiler bugs met on the way
    bench/           the vec3 loop against C behind FRICTION.md F21
    probes/          the small experiments FRICTION.md entries cite
    check.sh         unit tests, then every fixture on all three engines
    FRICTION.md      the journal of where kanso got in the way

## Running the checks

    sh check.sh

runs `kanso test` on every module, builds a dev-tier and a release binary,
and runs every fixture on the interpreter and both binaries, comparing each
run's stdout and stderr (with its exit status) to the expected files and to
each other. `sh check.sh --update` rewrites the expected files from the
interpreter. The fixture images are small (up to 48 pixels wide) so the
interpreter renders each in a few seconds.

Set `KANSO` to use a compiler other than `/tmp/claude-0/kanso-main/kanso`.
