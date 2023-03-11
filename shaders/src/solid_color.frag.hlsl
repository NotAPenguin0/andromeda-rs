struct PS_INPUT {
    [[vk::location(0)]] float2 UV : UV0;
    [[vk::location(1)]] float3 Color : COLOR0;
};

float4 main(PS_INPUT input) : SV_TARGET {
    return float4(input.Color, 1.0);
}