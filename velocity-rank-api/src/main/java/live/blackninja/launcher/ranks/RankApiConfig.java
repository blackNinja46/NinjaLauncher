package live.blackninja.launcher.ranks;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Locale;
import java.util.Properties;
import java.util.Set;
import java.util.stream.Collectors;

final class RankApiConfig {
    private static final String FILE_NAME = "config.properties";

    final InetAddress bindAddress;
    final int port;
    final String basePath;
    final int requestsPerMinute;
    final Set<String> hiddenGroups;

    private RankApiConfig(
            InetAddress bindAddress,
            int port,
            String basePath,
            int requestsPerMinute,
            Set<String> hiddenGroups
    ) {
        this.bindAddress = bindAddress;
        this.port = port;
        this.basePath = basePath;
        this.requestsPerMinute = requestsPerMinute;
        this.hiddenGroups = hiddenGroups;
    }

    static RankApiConfig load(Path dataDirectory) throws IOException {
        Files.createDirectories(dataDirectory);
        Path file = dataDirectory.resolve(FILE_NAME);
        Properties values = defaults();
        if (Files.isRegularFile(file)) {
            try (InputStream input = Files.newInputStream(file)) {
                values.load(input);
            }
        } else {
            try (OutputStream output = Files.newOutputStream(file)) {
                values.store(output, "Ninja Launcher rank API - keep the bind address on loopback");
            }
        }

        String path = values.getProperty("base-path", "/api/launcher/ranks").trim();
        if (!path.startsWith("/")) path = "/" + path;
        while (path.endsWith("/") && path.length() > 1) path = path.substring(0, path.length() - 1);

        Set<String> hidden = Arrays.stream(values.getProperty("hidden-groups", "default").split(","))
                .map(String::trim)
                .filter(value -> !value.isEmpty())
                .map(value -> value.toLowerCase(Locale.ROOT))
                .collect(Collectors.toUnmodifiableSet());

        int port = Integer.parseInt(values.getProperty("port", "8765"));
        int limit = Integer.parseInt(values.getProperty("requests-per-minute", "90"));
        if (port < 1 || port > 65535) throw new IOException("port must be between 1 and 65535");
        if (limit < 1 || limit > 10_000) throw new IOException("requests-per-minute must be between 1 and 10000");

        return new RankApiConfig(
                InetAddress.getByName(values.getProperty("bind-address", "127.0.0.1")),
                port,
                path,
                limit,
                hidden
        );
    }

    private static Properties defaults() {
        Properties values = new Properties();
        values.setProperty("bind-address", "127.0.0.1");
        values.setProperty("port", "8765");
        values.setProperty("base-path", "/api/launcher/ranks");
        values.setProperty("requests-per-minute", "90");
        values.setProperty("hidden-groups", "default");
        return values;
    }
}
