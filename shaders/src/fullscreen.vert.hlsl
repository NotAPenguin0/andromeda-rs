struct VSInput {

};

struct VSOutput {
    float4 Position : SV_POSITION;
    [[vk::location(0)]]
    float2 UV : UV0;
};


VSOutput main(VSInput input, uint VertexIndex : SV_VertexID) {
    // Vertices for fullscreen quad
    float4 vertices[] = {
        float4(-1.0, 1.0, 0.0, 1.0),
        float4(-1.0, -1.0, 0.0, 0.0),
        float4(1.0, -1.0, 1.0, 0.0),
        float4(-1.0, 1.0, 0, 1.0),
        float4(1.0, -1.0, 1.0, 0.0),
        float4(1.0, 1.0, 1.0, 1.0)
    };

    VSOutput output = (VSOutput)0;
    output.UV = vertices[VertexIndex].zw;
    // For simple '2D' passes the z and w coordinates wont matter, but for the atmosphere pass we want this to be behind
    // all other geometry, so we set z = w = 1
    output.Position = float4(vertices[VertexIndex].xy, 1.0, 1.0);
    return output;
}