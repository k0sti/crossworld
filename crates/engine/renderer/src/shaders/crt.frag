#version 300 es
precision highp float;

// CRT Post-Processing Fragment Shader
// Based on https://github.com/gingerbeardman/webgl-crt-shader
// Simulates CRT monitor effects: scanlines, curvature, bloom, vignette, RGB shift

uniform sampler2D tDiffuse;
uniform float scanlineIntensity;
uniform float scanlineCount;
uniform float time;
uniform float yOffset;
uniform float brightness;
uniform float contrast;
uniform float saturation;
uniform float bloomIntensity;
uniform float bloomThreshold;
uniform float rgbShift;
uniform float adaptiveIntensity;
uniform float vignetteStrength;
uniform float curvature;
uniform float flickerStrength;

in vec2 vUv;
out vec4 fragColor;

// Optimized curvature function using dot product
vec2 curveRemapUV(vec2 uv, float curveAmount) {
    vec2 coords = uv * 2.0 - 1.0;
    float curveScale = curveAmount * 0.25;
    float dist = dot(coords, coords);
    coords = coords * (1.0 + dist * curveScale);
    return coords * 0.5 + 0.5;
}

// Optimized bloom sampling (2x2 instead of 3x3)
vec4 sampleBloom(sampler2D tex, vec2 uv, float radius) {
    vec4 bloom = texture(tex, uv) * 0.4;
    bloom += texture(tex, uv + vec2(radius, 0.0)) * 0.2;
    bloom += texture(tex, uv + vec2(-radius, 0.0)) * 0.2;
    bloom += texture(tex, uv + vec2(0.0, radius)) * 0.2;
    return bloom;
}

// Vignette using Chebyshev distance approximation
float vignetteApprox(vec2 uv, float strength) {
    vec2 vigCoord = uv * 2.0 - 1.0;
    float dist = max(abs(vigCoord.x), abs(vigCoord.y));
    return 1.0 - dist * dist * strength;
}

void main() {
    vec2 uv = vUv;

    // Apply screen curvature
    if (curvature > 0.0) {
        uv = curveRemapUV(uv, curvature);
        // Black out pixels outside curved screen bounds
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
            fragColor = vec4(0.0, 0.0, 0.0, 1.0);
            return;
        }
    }

    // Sample base pixel
    vec4 pixel = texture(tDiffuse, uv);

    // Apply bloom effect
    if (bloomIntensity > 0.0) {
        float pixelLum = dot(pixel.rgb, vec3(0.299, 0.587, 0.114));
        if (pixelLum > bloomThreshold * 0.5) {
            vec4 bloomSample = sampleBloom(tDiffuse, uv, 0.005);
            bloomSample.rgb *= brightness;
            float bloomLum = dot(bloomSample.rgb, vec3(0.299, 0.587, 0.114));
            float bloomFactor = bloomIntensity * max(0.0, (bloomLum - bloomThreshold) * 1.5);
            pixel.rgb += bloomSample.rgb * bloomFactor;
        }
    }

    // RGB chromatic aberration shift
    if (rgbShift > 0.001) {
        float shift = rgbShift * 0.005;
        pixel.r += texture(tDiffuse, vec2(uv.x + shift, uv.y)).r * 0.08;
        pixel.b += texture(tDiffuse, vec2(uv.x - shift, uv.y)).b * 0.08;
    }

    // Brightness adjustment
    pixel.rgb *= brightness;

    // Contrast adjustment
    pixel.rgb = (pixel.rgb - 0.5) * contrast + 0.5;

    // Saturation adjustment
    float luminance = dot(pixel.rgb, vec3(0.299, 0.587, 0.114));
    pixel.rgb = mix(vec3(luminance), pixel.rgb, saturation);

    // Scanline effect
    float scanline = 1.0;
    if (scanlineIntensity > 0.0) {
        float scanlineY = (uv.y + yOffset) * scanlineCount;
        float scanlinePattern = abs(sin(scanlineY * 3.14159265));

        // Adaptive intensity based on Y position
        float adaptiveFactor = 1.0;
        if (adaptiveIntensity > 0.001) {
            float yPattern = sin(uv.y * 30.0) * 0.5 + 0.5;
            adaptiveFactor = 1.0 - yPattern * adaptiveIntensity * 0.2;
        }

        scanline = 1.0 - scanlinePattern * scanlineIntensity * adaptiveFactor;
    }

    // CRT flicker effect
    float flicker = 1.0 + sin(time * 110.0) * flickerStrength;

    // Vignette effect
    float vignette = 1.0;
    if (vignetteStrength > 0.0) {
        vignette = vignetteApprox(uv, vignetteStrength);
    }

    // Combine all lighting multipliers
    pixel.rgb *= scanline * flicker * vignette;

    fragColor = pixel;
}
