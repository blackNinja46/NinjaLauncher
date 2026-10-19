package live.blackninja.launcher.ranks;

import com.google.inject.Inject;
import com.sun.net.httpserver.Headers;
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.event.proxy.ProxyShutdownEvent;
import com.velocitypowered.api.plugin.Dependency;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.plugin.annotation.DataDirectory;
import net.luckperms.api.LuckPerms;
import net.luckperms.api.LuckPermsProvider;
import net.luckperms.api.cacheddata.CachedMetaData;
import net.luckperms.api.model.group.Group;
import net.luckperms.api.model.user.User;
import net.luckperms.api.query.QueryOptions;
import org.slf4j.Logger;

import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.regex.Pattern;

@Plugin(
        id = "ninja-launcher-rank-api",
        name = "Ninja Launcher Rank API",
        version = "1.0.0",
        description = "Read-only LuckPerms rank endpoint for Ninja Launcher",
        authors = {"BlackNinja.Live"},
        dependencies = {@Dependency(id = "luckperms")}
)
public final class RankApiPlugin {
    private static final Pattern LEGACY_COLOR = Pattern.compile("(?i)[&§][0-9A-FK-ORX]");
    private static final Pattern MINI_MESSAGE_TAG = Pattern.compile("<[^>]+>");
    private static final Pattern SAFE_COLOR = Pattern.compile("#[0-9a-fA-F]{6}");
    private static final Map<String, String> NAMED_COLORS = namedColors();

    private final Logger logger;
    private final Path dataDirectory;
    private final ConcurrentHashMap<String, RateWindow> rateLimits = new ConcurrentHashMap<>();
    private HttpServer httpServer;
    private LuckPerms luckPerms;
    private RankApiConfig config;

    @Inject
    public RankApiPlugin(Logger logger, @DataDirectory Path dataDirectory) {
        this.logger = logger;
        this.dataDirectory = dataDirectory;
    }

    @Subscribe
    public void onProxyInitialize(ProxyInitializeEvent event) {
        try {
            config = RankApiConfig.load(dataDirectory);
            luckPerms = LuckPermsProvider.get();
            httpServer = HttpServer.create(new InetSocketAddress(config.bindAddress, config.port), 0);
            httpServer.createContext(config.basePath, this::handleRankRequest);
            httpServer.setExecutor(Executors.newCachedThreadPool(runnable -> {
                Thread thread = new Thread(runnable, "ninja-rank-api");
                thread.setDaemon(true);
                return thread;
            }));
            httpServer.start();
            logger.info("Ninja Launcher rank API listening on http://{}:{}{}",
                    config.bindAddress.getHostAddress(), config.port, config.basePath + "/{uuid}");
        } catch (Exception exception) {
            logger.error("Could not start the Ninja Launcher rank API", exception);
        }
    }

    @Subscribe
    public void onProxyShutdown(ProxyShutdownEvent event) {
        if (httpServer != null) httpServer.stop(1);
    }

    private void handleRankRequest(HttpExchange exchange) throws IOException {
        try (exchange) {
            Headers headers = exchange.getResponseHeaders();
            headers.set("Content-Type", "application/json; charset=utf-8");
            headers.set("Cache-Control", "public, max-age=60");
            headers.set("X-Content-Type-Options", "nosniff");

            if (!"GET".equals(exchange.getRequestMethod())) {
                headers.set("Allow", "GET");
                send(exchange, 405, errorJson("method_not_allowed", "Only GET is supported."));
                return;
            }
            if (!allowRequest(exchange)) {
                headers.set("Retry-After", "60");
                send(exchange, 429, errorJson("rate_limited", "Too many requests."));
                return;
            }

            String path = exchange.getRequestURI().getPath();
            String prefix = config.basePath + "/";
            if (!path.startsWith(prefix) || path.length() <= prefix.length()) {
                send(exchange, 404, errorJson("not_found", "A player UUID is required."));
                return;
            }

            UUID playerId;
            try {
                playerId = parseUuid(path.substring(prefix.length()));
            } catch (IllegalArgumentException exception) {
                send(exchange, 400, errorJson("invalid_uuid", "The player UUID is invalid."));
                return;
            }

            User user;
            try {
                user = luckPerms.getUserManager().loadUser(playerId).get(5, TimeUnit.SECONDS);
            } catch (Exception exception) {
                logger.warn("Could not load LuckPerms user {}", playerId, exception);
                send(exchange, 503, errorJson("rank_service_unavailable", "Rank data is temporarily unavailable."));
                return;
            }

            List<RankView> ranks = resolveRanks(user);
            if (user.getUsername() == null && ranks.isEmpty()) {
                send(exchange, 404, errorJson("player_not_found", "No network profile exists for this player."));
                return;
            }
            send(exchange, 200, playerJson(user, ranks));
        }
    }

    private List<RankView> resolveRanks(User user) {
        QueryOptions queryOptions = luckPerms.getContextManager().getStaticQueryOptions();
        Map<String, Group> groups = new HashMap<>();
        for (Group group : user.getInheritedGroups(queryOptions)) groups.put(group.getName(), group);
        Group primary = luckPerms.getGroupManager().getGroup(user.getPrimaryGroup());
        if (primary != null) groups.putIfAbsent(primary.getName(), primary);

        List<RankView> ranks = new ArrayList<>();
        for (Group group : groups.values()) {
            if (config.hiddenGroups.contains(group.getName().toLowerCase(Locale.ROOT))) continue;
            CachedMetaData meta = group.getCachedData().getMetaData(queryOptions);
            String prefix = cleanText(meta.getPrefix());
            String displayName = firstNonBlank(
                    cleanDisplayLabel(meta.getMetaValue("rank-name")),
                    cleanDisplayLabel(meta.getMetaValue("display-name")),
                    cleanDisplayLabel(cleanRankPrefix(prefix)),
                    titleCase(group.getName())
            );
            ranks.add(new RankView(
                    group.getName(),
                    displayName,
                    prefix,
                    resolveRankIcon(group, meta),
                    normalizeColor(meta.getMetaValue("rank-color")),
                    group.getWeight().orElse(0),
                    group.getName().equalsIgnoreCase(user.getPrimaryGroup())
            ));
        }
        ranks.sort(Comparator.comparingInt(RankView::weight).reversed().thenComparing(RankView::name));
        return ranks;
    }

    private boolean allowRequest(HttpExchange exchange) {
        String client = exchange.getRemoteAddress().getAddress().getHostAddress();
        String forwarded = exchange.getRequestHeaders().getFirst("X-Real-IP");
        if (exchange.getLocalAddress().getAddress().isLoopbackAddress() && forwarded != null && !forwarded.isBlank()) {
            client = forwarded.trim();
        }
        long minute = Instant.now().getEpochSecond() / 60;
        return rateLimits.compute(client, (key, existing) -> {
            if (existing == null || existing.minute != minute) return new RateWindow(minute, 1);
            existing.count++;
            return existing;
        }).count <= config.requestsPerMinute;
    }

    private static UUID parseUuid(String value) {
        String clean = value.split("/", 2)[0].trim();
        if (clean.matches("[0-9a-fA-F]{32}")) {
            clean = clean.substring(0, 8) + "-" + clean.substring(8, 12) + "-" + clean.substring(12, 16)
                    + "-" + clean.substring(16, 20) + "-" + clean.substring(20);
        }
        return UUID.fromString(clean);
    }

    private static String playerJson(User user, List<RankView> ranks) {
        StringBuilder json = new StringBuilder(256);
        json.append("{\"uuid\":\"").append(user.getUniqueId()).append("\",")
                .append("\"username\":").append(jsonString(user.getUsername())).append(',')
                .append("\"primaryGroup\":").append(jsonString(user.getPrimaryGroup())).append(',')
                .append("\"ranks\":[");
        for (int index = 0; index < ranks.size(); index++) {
            if (index > 0) json.append(',');
            RankView rank = ranks.get(index);
            json.append("{\"name\":").append(jsonString(rank.name()))
                    .append(",\"displayName\":").append(jsonString(rank.displayName()))
                    .append(",\"prefix\":").append(jsonString(rank.prefix()))
                    .append(",\"icon\":").append(jsonString(rank.icon()))
                    .append(",\"color\":").append(jsonString(rank.color()))
                    .append(",\"weight\":").append(rank.weight())
                    .append(",\"primary\":").append(rank.primary()).append('}');
        }
        return json.append("]}").toString();
    }

    private static String errorJson(String code, String message) {
        return "{\"error\":" + jsonString(code) + ",\"message\":" + jsonString(message) + "}";
    }

    private static String jsonString(String value) {
        if (value == null) return "null";
        StringBuilder escaped = new StringBuilder(value.length() + 8).append('"');
        for (int index = 0; index < value.length(); index++) {
            char character = value.charAt(index);
            switch (character) {
                case '"' -> escaped.append("\\\"");
                case '\\' -> escaped.append("\\\\");
                case '\n' -> escaped.append("\\n");
                case '\r' -> escaped.append("\\r");
                case '\t' -> escaped.append("\\t");
                default -> {
                    if (character < 0x20) escaped.append(String.format("\\u%04x", (int) character));
                    else escaped.append(character);
                }
            }
        }
        return escaped.append('"').toString();
    }

    private static void send(HttpExchange exchange, int status, String body) throws IOException {
        byte[] response = body.getBytes(StandardCharsets.UTF_8);
        exchange.sendResponseHeaders(status, response.length);
        try (OutputStream output = exchange.getResponseBody()) {
            output.write(response);
        }
    }

    private static String cleanText(String value) {
        if (value == null) return "";
        return MINI_MESSAGE_TAG.matcher(LEGACY_COLOR.matcher(value).replaceAll("")).replaceAll("").trim();
    }

    private static String cleanRankPrefix(String prefix) {
        return prefix.replaceAll("^\\[|]$", "").trim();
    }

    private static String cleanDisplayLabel(String value) {
        String cleaned = cleanText(value);
        if (cleaned.length() > 32 || cleaned.codePoints().noneMatch(Character::isLetterOrDigit)) return "";
        return cleaned;
    }

    private static String resolveRankIcon(Group group, CachedMetaData meta) {
        String encoded = String.join(" ",
                valueOrEmpty(meta.getMetaValue("rank-name")),
                valueOrEmpty(meta.getMetaValue("display-name")),
                valueOrEmpty(meta.getPrefix()));
        Map<String, String> glyphs = Map.ofEntries(
                Map.entry("⺟", "supporter"), Map.entry("⻄", "content"),
                Map.entry("⿔", "builder"), Map.entry("⻑", "player"),
                Map.entry("⻦", "ninja"), Map.entry("⪞", "vip"),
                Map.entry("⫸", "moderator"), Map.entry("⏅", "admin"),
                Map.entry("⚺", "cam")
        );
        for (Map.Entry<String, String> glyph : glyphs.entrySet()) {
            if (encoded.contains(glyph.getKey())) return glyph.getValue();
        }
        String name = group.getName().toLowerCase(Locale.ROOT).replaceAll("[^a-z]", "");
        if (glyphs.containsValue(name)) return name;
        return switch (name) {
            case "mod" -> "moderator";
            case "con", "creator" -> "content";
            case "sup" -> "supporter";
            default -> null;
        };
    }

    private static String valueOrEmpty(String value) {
        return value == null ? "" : value;
    }

    private static String normalizeColor(String color) {
        if (color == null) return null;
        String normalized = color.trim().toLowerCase(Locale.ROOT);
        if (SAFE_COLOR.matcher(normalized).matches()) return normalized;
        return NAMED_COLORS.get(normalized.replace('-', '_'));
    }

    private static String firstNonBlank(String... values) {
        for (String value : values) if (value != null && !value.isBlank()) return value;
        return "Rank";
    }

    private static String titleCase(String value) {
        if (value.isBlank()) return value;
        return Character.toUpperCase(value.charAt(0)) + value.substring(1).replace('_', ' ');
    }

    private static Map<String, String> namedColors() {
        return Map.ofEntries(
                Map.entry("black", "#000000"), Map.entry("dark_blue", "#0000aa"),
                Map.entry("dark_green", "#00aa00"), Map.entry("dark_aqua", "#00aaaa"),
                Map.entry("dark_red", "#aa0000"), Map.entry("dark_purple", "#aa00aa"),
                Map.entry("gold", "#ffaa00"), Map.entry("gray", "#aaaaaa"),
                Map.entry("dark_gray", "#555555"), Map.entry("blue", "#5555ff"),
                Map.entry("green", "#55ff55"), Map.entry("aqua", "#55ffff"),
                Map.entry("red", "#ff5555"), Map.entry("light_purple", "#ff55ff"),
                Map.entry("yellow", "#ffff55"), Map.entry("white", "#ffffff")
        );
    }

    private record RankView(String name, String displayName, String prefix, String icon, String color, int weight, boolean primary) {}

    private static final class RateWindow {
        private final long minute;
        private int count;

        private RateWindow(long minute, int count) {
            this.minute = minute;
            this.count = count;
        }
    }
}
