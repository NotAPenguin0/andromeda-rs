struct PS_INPUT {
    [[vk::location(0)]] float2 UV : UV0;
    [[vk::location(1)]] float3 Normal : NORMAL0;
};

float4 main(PS_INPUT input) : SV_TARGET {
    float4 _unused = float4(input.UV, 0.0, 0.0) * 0.0001;
    float4 norm = float4(input.Normal * 0.5 + float3(0.5, 0.5, 0.5), 1.0) +_unused;
    return pow(norm, 2.2);
}