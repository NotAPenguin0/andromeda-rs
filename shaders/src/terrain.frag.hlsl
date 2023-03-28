struct PS_INPUT {
    [[vk::location(0)]] float2 UV : UV0;
};

float4 main(PS_INPUT input) : SV_TARGET {
    return float4(input.UV, 0.0, 1.0);
}