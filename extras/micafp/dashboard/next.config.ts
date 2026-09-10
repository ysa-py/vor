import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // React 19 strict mode
  reactStrictMode: true,

  // Allow the Freebuff/Daytona preview proxy origin to load dev resources
  allowedDevOrigins: ["**.daytonaproxy01.net"],

  // API routes configuration
  async headers() {
    return [
      {
        source: "/api/:path*",
        headers: [
          { key: "Access-Control-Allow-Origin", value: "*" },
          { key: "Access-Control-Allow-Methods", value: "GET,POST,PUT,DELETE,OPTIONS" },
          { key: "Access-Control-Allow-Headers", value: "Content-Type, Authorization" },
        ],
      },
    ];
  },

  // Redirect HTTP to HTTPS in production
  async redirects() {
    return process.env.NODE_ENV === "production"
      ? [
          {
            source: "/:path*",
            has: [
              {
                type: "header",
                key: "x-forwarded-proto",
                value: "http",
              },
            ],
            permanent: true,
            destination: "https://:host/:path*",
          },
        ]
      : [];
  },

  // Image domains - Chinese CDNs are primary (Cloudflare BLOCKED in Iran)
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "unifiedshield.oss-cn-shanghai.aliyuncs.com",
      },
      {
        protocol: "https",
        hostname: "unifiedshield.oss-cn-hongkong.aliyuncs.com",
      },
      {
        protocol: "https",
        hostname: "unifiedshield-1258344699.cos.ap-hongkong.myqcloud.com",
      },
      {
        protocol: "https",
        hostname: "avatars.githubusercontent.com",
      },
    ],
  },
};

export default nextConfig;
