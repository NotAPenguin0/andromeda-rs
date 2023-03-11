float rgb2lum(float3 rgb) {
    return 0.2125 * rgb.r + 0.7154 * rgb.g + 0.0721 * rgb.b;
}

// Conversions below are adapted from https://github.com/tobspr/GLSL-Color-Spaces/blob/master/ColorSpaces.inc.glsl

static float SRGB_GAMMA = 1.0 / 2.2;
static float SRGB_INVERSE_GAMMA = 2.2;
static float SRGB_ALPHA = 0.055;


// Used to convert from linear RGB to XYZ space
static float3x3 RGB_2_XYZ = (float3x3(
    0.4124564, 0.2126729, 0.0193339,
    0.3575761, 0.7151522, 0.1191920,
    0.1804375, 0.0721750, 0.9503041
));

// Used to convert from XYZ to linear RGB space
static float3x3 XYZ_2_RGB = (float3x3(
    3.2404542,-0.9692660, 0.0556434,
    -1.5371385, 1.8760108,-0.2040259,
    -0.4985314, 0.0415560, 1.0572252
));

// Converts a linear rgb color to a srgb color (approximated, but fast)
float3 rgb2srgb_approx(float3 rgb) {
    return pow(rgb, float3(SRGB_GAMMA, SRGB_GAMMA, SRGB_GAMMA));
}

// Converts a srgb color to a rgb color (approximated, but fast)
float3 srgb2rgb_approx(float3 srgb) {
    return pow(srgb, float3(SRGB_INVERSE_GAMMA, SRGB_INVERSE_GAMMA, SRGB_INVERSE_GAMMA));
}

// Converts a single linear channel to srgb
float linear2srgb(float channel) {
    if(channel <= 0.0031308)
    return 12.92 * channel;
    else
    return (1.0 + SRGB_ALPHA) * pow(channel, 1.0/2.4) - SRGB_ALPHA;
}

// Converts a single srgb channel to rgb
float srgb2linear(float channel) {
    if (channel <= 0.04045)
    return channel / 12.92;
    else
    return pow((channel + SRGB_ALPHA) / (1.0 + SRGB_ALPHA), 2.4);
}

// Converts a linear rgb color to a srgb color (exact, not approximated)
float3 rgb2srgb(float3 rgb) {
    return float3(
        linear2srgb(rgb.r),
        linear2srgb(rgb.g),
        linear2srgb(rgb.b)
    );
}

// Converts a srgb color to a linear rgb color (exact, not approximated)
float3 srgb2rgb(float3 srgb) {
    return float3(
        srgb2linear(srgb.r),
        srgb2linear(srgb.g),
        srgb2linear(srgb.b)
    );
}

// Converts a color from linear RGB to XYZ space
float3 rgb2xyz(float3 rgb) {
    return mul(RGB_2_XYZ, rgb);
}

// Converts a color from XYZ to linear RGB space
float3 xyz2rgb(float3 xyz) {
    return mul(XYZ_2_RGB, xyz);
}

// Converts a color from XYZ to xyY space (Y is luminosity)
float3 xyz2xyY(float3 xyz) {
    float Y = xyz.y;
    float x = xyz.x / (xyz.x + xyz.y + xyz.z);
    float y = xyz.y / (xyz.x + xyz.y + xyz.z);
    return float3(x, y, Y);
}

// Converts a color from xyY space to XYZ space
float3 xyY2xyz(float3 xyY) {
    float Y = xyY.z;
    float x = Y * xyY.x / xyY.y;
    float z = Y * (1.0 - xyY.x - xyY.y) / xyY.y;
    return float3(x, Y, z);
}

// Converts a color from linear RGB to xyY space
float3 rgb2xyY(float3 rgb) {
    float3 xyz = rgb2xyz(rgb);
    return xyz2xyY(xyz);
}

// Converts a color from xyY space to linear RGB
float3 xyY2rgb(float3 xyY) {
    float3 xyz = xyY2xyz(xyY);
    return xyz2rgb(xyz);
}

// To srgb
float3 xyz2srgb(float3 xyz)  { return rgb2srgb(xyz2rgb(xyz)); }
float3 xyY2srgb(float3 xyY)  { return rgb2srgb(xyY2rgb(xyY)); }

float3 srgb2xyz(float3 srgb) { return rgb2xyz(srgb2rgb(srgb)); }

float3 srgb2xyY(float3 srgb) { return rgb2xyY(srgb2rgb(srgb)); }