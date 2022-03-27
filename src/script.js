const IMAGE_REGEX = new RegExp('src=\"https:\\/\\/.+media.tumblr\\.com.+\"');

$( document ).ready(function() {
    const BLOG_CHOICE = $("#blog-choice");
    const SEARCH = $("#search");
    const PAGE_CHOICE = $("#page-choice");
    const FORM = $("#form")
    const POSTS_DIV = $("#posts");
    const PAGE_SIZE = 100;
    const TYPE = $("#type")
    const TOTAL = $("#total")
    const SHOWING = $("#showing")

    let ALL_POSTS = [];
    let FILTERED_POSTS = [];

    FORM.trigger("reset");
    FORM.submit(function( event ) {
        event.preventDefault();
    });
    SEARCH.attr('disabled' , true);

    $.get( "/blogs/" ).then(
        function(res) {
            parse_blogs_list(res);
        },
        function (e) {
            throw new Error("Get /blogs/ failed")
        }
    ).catch((e) => {
        alert(e);
    })

    // Convert nginx directory index into list of blogs
    function parse_blogs_list(res) {
        const html = $.parseHTML( res );
        const array = Array.from(html[5].children).slice(1)
        const mapped = array.map((e) => e.text.slice(0, -1))
        const filtered = mapped.filter((e) => e !== "Index")
        if (filtered.length < 1) {
            throw new Error("No blogs found")
        }
        BLOG_CHOICE.empty();
        const placeholder = new Option("Choose a blog", "");
        placeholder.setAttribute('disabled', true);
        placeholder.setAttribute('selected', true);
        BLOG_CHOICE.append(placeholder);
        filtered.forEach((d) => BLOG_CHOICE.append(new Option(d,d)));
        BLOG_CHOICE.attr('disabled' , false);
    }

    BLOG_CHOICE.change(function() {
        const blog = $(this).val();
        const requests = [
            Image.load(blog),
            Text.load(blog),
            Answer.load(blog),
            Video.load(blog),
        ]
        $.when(...requests).then((...responses) => {
            ALL_POSTS = responses.flat()
            TOTAL.text(`Total: ${ALL_POSTS.length}`)
            apply_filters();
            update_page_choice();
            render_posts();
            SEARCH.attr('disabled' , false);
            TYPE.attr('disabled' , false);
        }).catch((e) => {
            alert(e);
        })
    });

    PAGE_CHOICE.change(function() {
        apply_filters();
        render_posts();
    });

    TYPE.change(function() {
        refresh()
    });

    SEARCH.on("input", function(e) {
        clearTimeout(this.thread);
        this.thread = setTimeout(function() {
            refresh()
        }, 150);
    });

    function refresh() {
        apply_filters();
        update_page_choice();
        render_posts();
    }

    // Filters by search and selected page number
    function apply_filters() {
        const search = SEARCH[0].value;
        const type = TYPE[0].value;
        FILTERED_POSTS = ALL_POSTS.filter((p) => {
            return type === "All" || type === p.constructor.name
        })
        FILTERED_POSTS = FILTERED_POSTS.filter((p) => {
            return search.length === 0 || p.matches_search(search)
        })
        FILTERED_POSTS.sort(function (a, b) {
            return a.id - b.id;
        });
    }

    function update_page_choice() {
        PAGE_CHOICE.empty()
        if (FILTERED_POSTS.length < 1) {
            const placeholder = new Option("1", "1");
            placeholder.setAttribute('disabled', true);
            PAGE_CHOICE.append(placeholder)
            PAGE_CHOICE.attr('disabled' , true);
        } else {
            const pages = Math.ceil(FILTERED_POSTS.length / PAGE_SIZE)
            for (let i = 1; i <= pages; i++) {
                PAGE_CHOICE.append(new Option(i.toString(), i.toString()));
            }
            PAGE_CHOICE.attr('disabled' , false);
        }
    }

    function render_posts() {
        POSTS_DIV.empty();
        const page_number = parseInt(PAGE_CHOICE[0].value) - 1;
        const start = page_number * PAGE_SIZE
        const stop = (page_number + 1) * PAGE_SIZE
        const posts = FILTERED_POSTS.slice(start, stop);
        SHOWING.text(`Showing: ${posts.length}`)
        for (const post of posts) {
            const render = post.render();
            const type = post.constructor.name
            POSTS_DIV.append(`<div class='post ${type}' id="${post.id}">${render}</div>`)
        }
    }

});

class Post {
    id;
    date;
    tags;

    static POST_ID = "Post id: "
    static DATE = "Date: "
    static POST_URL = "Post url: "
    static SLUG = "Slug: "
    static REBLOG_KEY = "Reblog key: "
    static REBLOG_URL = "Reblog url: "
    static REBLOG_NAME = "Reblog name: "
    static TAGS = "Tags: "

    constructor(id, date, tags) {
        this.id = id
        this.date = date
        this.tags = tags
    }

    static deserialize_from_text(lines) {
        const id = parseInt(Post.line_starting_with(lines, Post.POST_ID));
        const date = Post.line_starting_with(lines, Post.DATE);
        const tags = Post.line_starting_with(lines, Post.TAGS);
        return [id, date, tags];
    }

    static deserialize_from_json(json) {
        const id = parseInt(json.id);
        const date = json.date;
        const tags = json.tags.join(", ");
        return [id, date, tags];
    }

    // Returns the contents or null if it does not exist
    static get_blog_file(blog, file) {
        const path = `/blogs/${blog}/${file}`;
        return $.get(path).catch((res) => {
            if (res.status === 404) {
                return null
            }
            throw new Error(`Error getting blog file: ${path} code: ${res.status}`)
        })
    }

    // Returns an array of posts
    // Where each post is an array of strings (lines)
    static split_posts_in_text(all_posts) {
        const lines = all_posts.split(/\r?\n/);
        const out = []
        var buffer = [];
        for (const line of lines) {
            if (line.startsWith(Post.POST_ID)) {
                out.push([...buffer])
                buffer = []
            }
            buffer.push(line)
        }
        out.push([...buffer])
        return out.filter((inner) => inner.length > 0)
    }

    // Given a post (array of lines), find the contents between two identifiers
    // Set right as null to collect all remaining lines
    static contents_between_lines(lines, left, right) {
        const subset = []
        var hit = false
        for (const line of lines) {
            // Break if we hit the end
            if (line.startsWith(right)) {
                break
            }
            // Catch the lines in between
            if (hit) {
                subset.push(line)
            }
            // Catch the first line
            if (line.startsWith(left)) {
                hit = true
            }
        }
        return subset.join("\n")
    }

    static line_starting_with(lines, id) {
        for (const line of lines) {
            if (line.startsWith(id)) {
                return line.slice(id.length)
            }
        }
        return null
    }

    static fix_url(url, blog) {
        let idx = url.lastIndexOf('/')
        let subst = url.slice(idx + 1)
        return `/blogs/${blog}/${subst}`
    }

    render_header() {
        return `<p>${this.date}</p>`
    }

    render_footer() {
        return `<p>Tags: ${this.tags}</p>`
    }

    matches_search(search) {
        return this.tags.includes(search)
    }

}

class Image extends Post {
    photo_urls;
    caption;

    static PHOTO_URL = "Photo url: "
    static PHOTO_SET_URLS = "Photo set urls: "
    static PHOTO_CAPTION = "Photo caption: "

    constructor(id, date, tags, photo_urls, caption) {
        super(id, date, tags);
        this.photo_urls = photo_urls
        this.caption = caption
    }

    static deserialize_from_text(lines, blog) {
        const [id, date, tags] = Post.deserialize_from_text(lines)
        const photo_set_urls = Post.line_starting_with(lines, Image.PHOTO_SET_URLS).split(" ").filter((u) => u.length > 0)
        var photo_urls
        if (photo_set_urls.length > 0) {
            photo_urls = photo_set_urls
        } else {
            photo_urls = [Post.line_starting_with(lines, Image.PHOTO_URL)]
        }
        photo_urls = photo_urls.map((u) => Post.fix_url(u, blog))
        const caption = Post.line_starting_with(lines, Image.PHOTO_CAPTION)
        return new Image(id, date, tags, photo_urls, caption)
    }

    static deserialize_from_json(json, blog) {
        const [id, date, tags] = Post.deserialize_from_json(json)
        const urls = json.post_html.match(IMAGE_REGEX).map((u) => Post.fix_url(u.slice(1, -1), blog))
        return new Image(id, date, tags, urls, json.caption)
    }

    static load(blog) {
        return Post.get_blog_file(blog, "images.txt").then((res) => {
            return convert_posts(res,
                (lines) => Image.deserialize_from_text(lines, blog),
                (json) => Image.deserialize_from_json(json, blog),
                )
        })
    }

    render() {
        const header = super.render_header();
        const images = this.photo_urls.map((u) => `<img src="${u}" alt="">`).join("\n")
        const footer = super.render_footer();
        return [header, this.caption, images, footer].join("\n")
    }

    matches_search(search) {
        return super.matches_search(search) || this.caption.includes(search)
    }

}

class Video extends Post {
    url;
    caption;

    static VIDEO_CAPTION = "Video caption: "
    static VIDEO_PLAYER = "Video player: "

    constructor(id, date, tags, url, caption) {
        super(id, date, tags)
        this.url = url
        this.caption = caption
    }

    static deserialize_from_text(lines, blog) {
        const [id, date, tags] = Post.deserialize_from_text(lines)
        const player = Post.contents_between_lines(lines, Video.VIDEO_PLAYER);
        const html = $.parseHTML( player );
        let url = html[0].children[0].attributes["src"].value;
        url = Video.fix_url(url, blog)
        const caption = Post.line_starting_with(lines, Video.VIDEO_CAPTION);
        return new Video(id, date, tags, url, caption)
    }

    static deserialize_from_json(json, blog) {
        const [id, date, tags] = Post.deserialize_from_json(json);
        const url = Post.fix_url(json.video_url, blog);
        const caption = json.caption;
        return new Video(id, date, tags, url, caption)
    }

    static fix_url(url, blog) {
        const baseFix = Post.fix_url(url, blog)
        const lastDot = baseFix.lastIndexOf('.')
        const lastUnderscore = baseFix.lastIndexOf('_')
        const left = baseFix.slice(0, lastUnderscore)
        const right = baseFix.slice(lastDot)
        if (left.endsWith("tumblr")) {
            return baseFix
        }
        return left + right
    }

    static load(blog) {
        return Post.get_blog_file(blog, "videos.txt").then((res) => {
            return convert_posts(res,
                (lines) => Video.deserialize_from_text(lines, blog),
                (json) => Video.deserialize_from_json(json, blog),
                )
        })
    }

    render() {
        const header = super.render_header();
        const footer = super.render_footer();
        const caption = `<div>${this.caption}</div>`
        const video = `<video controls><source src="${this.url}"></video>`
        return [header, caption, video, footer].join("\n")
    }

    matches_search(search) {
        return super.matches_search(search) || this.caption.includes(search)
    }
}

class Text extends Post {
    title;
    body;

    static TITLE = "Title: "

    constructor(id, date, tags, title, body) {
        super(id, date, tags);
        this.title = title
        this.body = body
    }

    static deserialize_from_text(lines) {
        const [id, date, tags] = Post.deserialize_from_text(lines)
        const title = Post.line_starting_with(lines, Text.TITLE)
        const body = Post.contents_between_lines(lines, Text.TITLE, Post.TAGS)
        return new Text(id, date, tags, title, body)
    }

    static deserialize_from_json(json, blog) {
        const [id, date, tags] = Post.deserialize_from_json(json)
        return new Text(id, date, tags)
    }

    static load(blog) {
        return Post.get_blog_file(blog, "texts.txt").then((res) => {
            return convert_posts(res,
                (lines) => Text.deserialize_from_text(lines, blog),
                (json) => Text.deserialize_from_json(json, blog),
                )
        })
    }

    render() {
        const header = super.render_header();
        const footer = super.render_footer();
        const title = `<h4>${this.title}</h4>`
        return [header, title, this.body, footer].join("\n")
    }

    matches_search(search) {
        return super.matches_search(search) || this.title.includes(search) || this.body.includes(search)
    }

}

class Answer extends Post {
    body;

    constructor(id, date, tags, body) {
        super(id, date, tags);
        this.body = body
    }

    static deserialize_from_text(lines, blog) {
        const [id, date, tags] = Post.deserialize_from_text(lines)
        const body = Post.contents_between_lines(lines, Post.REBLOG_NAME, Post.TAGS)
        return new Answer(id, date, tags, body)
    }

    static deserialize_from_json(json, blog) {
        const [id, date, tags] = Post.deserialize_from_json(json)
        return new Answer(id, date, tags)
    }

    static load(blog) {
        return Post.get_blog_file(blog, "answers.txt").then((res) => {
            return convert_posts(res,
                (lines) => Answer.deserialize_from_text(lines, blog),
                (json) => Answer.deserialize_from_json(json, blog),
                )
        })
    }

    render() {
        const header = super.render_header();
        const footer = super.render_footer();
        return [header, this.body, footer].join("\n")
    }

    matches_search(search) {
        return super.matches_search(search) || this.body.includes(search)
    }

}

function convert_posts(text, text_mapper, json_mapper, context) {
    let posts = []
    if (!text) {
        return posts
    }
    let split, mapper
    if (text.startsWith("[")) {
        split = JSON.parse(text)
        mapper = json_mapper
    } else {
        split = Post.split_posts_in_text(text);
        mapper = text_mapper
    }
    for (const post of split) {
        try {
            posts.push(mapper(post))
        } catch (e) {
            console.error(`Unable to convert post: due to: ${e}`)
            console.error(post)
        }
    }
    return posts
}
