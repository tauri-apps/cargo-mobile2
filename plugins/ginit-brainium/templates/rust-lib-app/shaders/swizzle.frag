#version 450

layout(std430) uniform;

uniform sampler2D texSampler;

in vec4 fragColor;
in vec2 fragTexCoord;
out vec4 outColor;

void main() {
    outColor = (texture(texSampler, fragTexCoord) * fragColor).brga;
}
