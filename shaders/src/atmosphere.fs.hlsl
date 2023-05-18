static const uint SAMPLES = 16;
static const uint TRANSMITTANCE_SAMPLES = 8;
static const float PI = 3.1415926535897932384626433832;

struct Atmosphere {
    float4 radii_mie_albedo_g; // x = planet radius, y = atmosphere radius, z = mie albedo, w = mie asymmetry parameter (g)
    float4 rayleigh; // xyz = coefficients, w = scatter height
    float4 mie; // xyz = coefficients, w = scatter height
    float4 ozone_sun; // xyz = ozone coeff, w = sun intensity
};

struct AtmosphereParameters {
    float planet_radius;
    float atmosphere_radius;

    float3 rayleigh_coeff;
    float rayleigh_scatter_height;

    float3 mie_coeff;
    float mie_albedo;
    float mie_scatter_height;
    float mie_g;

    float3 ozone_coeff;

    float3x2 scatter_coeff;
    float3x3 extinction_coeff;

    float sun_illuminance;
};

[[vk::push_constant]]
struct PC {
    // Direction away from the sun (verify!)
    float4 sun_dir;
} pc;

[[vk::binding(0, 0)]]
cbuffer camera {
    float4x4 projection_view;
    float4x4 inv_projection;
    float4x4 inv_view_rotation;
    float4 cam_position;
}

[[vk::binding(1, 0)]]
cbuffer atmosphere_settings {
    Atmosphere atm_params;
}

AtmosphereParameters get_atmosphere_params(Atmosphere atm) {
    AtmosphereParameters a;
    a.planet_radius = atm.radii_mie_albedo_g.x;
    a.atmosphere_radius = atm.radii_mie_albedo_g.y;
    a.rayleigh_coeff = atm.rayleigh.xyz;
    a.rayleigh_scatter_height = atm.rayleigh.w;
    a.mie_coeff = atm.mie.xyz;
    a.mie_scatter_height = atm.mie.w;
    a.mie_albedo = atm.radii_mie_albedo_g.z;
    a.mie_g = atm.radii_mie_albedo_g.w;
    a.ozone_coeff = atm.ozone_sun.xyz;
    a.sun_illuminance = atm.ozone_sun.w;
    // float3x2, but we want coefficients in the columns
    float3x2 scatter_coeff = {
        a.rayleigh_coeff.x, a.mie_coeff.x,
        a.rayleigh_coeff.y, a.mie_coeff.y,
        a.rayleigh_coeff.z, a.mie_coeff.z,
    };
    a.scatter_coeff = scatter_coeff;
    // a.scatter_coeff = float3x2(a.rayleigh_coeff, a.mie_coeff);
    
    float3 mie_coeff = a.mie_coeff / a.mie_albedo;
    a.extinction_coeff[0] = float3(a.rayleigh_coeff.x, mie_coeff.x, a.ozone_coeff.x);
    a.extinction_coeff[1] = float3(a.rayleigh_coeff.y, mie_coeff.y, a.ozone_coeff.y);
    a.extinction_coeff[2] = float3(a.rayleigh_coeff.z, mie_coeff.z, a.ozone_coeff.z);
    // a.extinction_coeff[0] = a.rayleigh_coeff;
    // a.extinction_coeff[1] = a.mie_coeff / a.mie_albedo;
    // a.extinction_coeff[2] = a.ozone_coeff;
    return a;
}

// get world position and convert to position relative to earth center
float3 relative_to_planet(AtmosphereParameters atm, float3 p) {
    return float3(p.x, p.y + atm.planet_radius, p.z);
}

// Add a small offset to the atmosphere ray to avoid self-intersecting with planet.
float3 add_ray_offset(float3 p) {
    return p + float3(0, 10, 0);
}

float3 camera_ray_direction(float2 uv) {
    uv = uv * 2.0 - 1.0;
    float4 target = mul(inv_projection, float4(uv.x, uv.y, 1, 1));
    return normalize(mul(inv_view_rotation, float4(normalize(target.xyz), 0))).xyz;
}

float2 ray_sphere_intersection(float3 origin, float3 direction, float radius) {
    float b = dot(origin, direction);
    float c = dot(origin, origin) - radius * radius;
    float d = b * b - c;

    if (d < 0.0)
        return -1.0.xx;

    d = sqrt(d);

    return float2(-b - d, -b + d);
}

float rayleigh_phase(float cos_theta) {
    return (3.0 / (16.0 * PI)) * (1.0 + cos_theta * cos_theta);
}

// cornette-shanks phase function
float mie_phase(float cos_theta, float g) {
    const float g_squared = g * g;
    const float p1 = 3.0 * (1.0 - g_squared) * (1.0 / (PI * (2.0 + g_squared)));
    const float p2 = (1.0 + (cos_theta * cos_theta)) * (1.0 / pow((1.0 + g_squared - 2.0 * g * cos_theta), 1.5));

    float phase = (p1 * p2);
    phase *= 0.25 / PI;

    return max(phase, 0.0);
}

float3 get_densities(AtmosphereParameters atm, float altitude) {
    float rayleigh = exp(-altitude / atm.rayleigh_scatter_height);
    float mie = exp(-altitude / atm.mie_scatter_height);
    float ozone = exp(-max(0.0, (35000 - altitude) - atm.atmosphere_radius) / 5000) * exp(-max(0.0, (altitude - 35000) - atm.atmosphere_radius) / 15000);

    return float3(rayleigh, mie, ozone);
}

// light direction must be towards the light source, not away from it.
float3 compute_light_transmittance(AtmosphereParameters atm, float3 position, float3 light_direction) {
    // get intersection with atmosphere
    float2 atm_intersect = ray_sphere_intersection(position, light_direction, atm.atmosphere_radius);
    const float dt = atm_intersect.y / float(TRANSMITTANCE_SAMPLES);
    const float3 dr = light_direction * dt;

    // initial ray position
    float3 ray_position = position + dr * 0.5;
    // initialize accumulator
    float3 total_transmittance = 1.0.xxx;

    for (int i = 0; i < TRANSMITTANCE_SAMPLES; ++i) {
        // TODO: same code as in get_sky_color, should move this to a function
        const float altitude = length(ray_position) - atm.planet_radius;
        const float3 density = get_densities(atm, altitude);
        const float3 air_mass = density * dt;
        const float3 optical_depth = mul(atm.extinction_coeff, air_mass);
        const float3 transmittance = exp(-optical_depth);
        total_transmittance *= transmittance;
        ray_position += dr;
    }
    return total_transmittance;
}

float3 get_sky_color(AtmosphereParameters atm, float3 ray_origin, float3 ray_direction, float3 light_direction) {
    // get intersection points with planet and atmosphere to determine step size.
    float2 atm_intersect = ray_sphere_intersection(ray_origin, ray_direction, atm.atmosphere_radius);
    float2 planet_intersect = ray_sphere_intersection(ray_origin, ray_direction, atm.planet_radius);

    bool planet_intersected = planet_intersect.y >= 0.0;
    bool atmosphere_intersected = atm_intersect.y >= 0.0;
    float2 sd = float2(
        (planet_intersected && planet_intersect.x < 0.0) ? planet_intersect.y : max(atm_intersect.x, 0.0),
        (planet_intersected && planet_intersect.x > 0.0) ? planet_intersect.x : atm_intersect.y);

    // compute step size
    float dt = length(sd.y - sd.x) / float(SAMPLES);
    // ray increment
    float3 dr = ray_direction * dt;

    // start position for ray
    float3 ray_position = ray_direction * sd.x + (dr * 0.5 + ray_origin);

    // initialize variables for accumulating scattering and transmittance
    float3 total_scattering = 0.0.xxx;
    float3 total_transmittance = 1.0.xxx;

    // compute rayleigh and mie phase functions
    float cos_theta = dot(ray_direction, light_direction);
    float2 phases = float2(rayleigh_phase(cos_theta), mie_phase(cos_theta, atm.mie_g));

    // now we will march along the path of the view ray to compute the sky color for this pixel.
    for (int i = 0; i < SAMPLES; ++i) {
        // compute altitude of this sample point, for sampling density functions.
        const float altitude = length(ray_position) - atm.planet_radius;
        // get density and air mass
        const float3 density = get_densities(atm, altitude);
        const float3 air_mass = density * dt;
        // with that we can compute the optical depth
        const float3 optical_depth = mul(atm.extinction_coeff, air_mass);
        // ... which is in turn the transmittance
        const float3 transmittance = exp(-optical_depth);
        // sample total transmittance along light ray (!= current ray)
        const float3 light_transmittance = compute_light_transmittance(atm, ray_position, light_direction);
        // single scattering
        const float3 scattering = mul(atm.scatter_coeff, (phases.xy * air_mass.xy)) * light_transmittance;
        // we can use this to solve scattering integral (method from frostbite: https://media.contentapi.ea.com/content/dam/eacom/frostbite/files/s2016-pbs-frostbite-sky-clouds-new.pdf)
        const float3 scattering_integral = (scattering - scattering * transmittance) / max(0.00000001.xxx, optical_depth);

        // accumulate variables and move ray
        total_scattering += scattering_integral * total_transmittance;
        total_transmittance *= transmittance;
        ray_position += dr;
    }

    return atm.sun_illuminance * total_scattering;
}

float4 main([[vk::location(0)]] in float2 UV : UV0) : SV_TARGET {
    AtmosphereParameters atm = get_atmosphere_params(atm_params);

    float3 view_pos = relative_to_planet(atm, cam_position.xyz);
    float3 ray_origin = add_ray_offset(view_pos);
    float3 ray_direction = camera_ray_direction(UV);
    // note that this shader expects light direction to be towards the light source instead of away from it
    float3 light_direction = -pc.sun_dir.xyz;

    float3 color = get_sky_color(atm, ray_origin, ray_direction, light_direction);
    return float4(color, 1.0);
}