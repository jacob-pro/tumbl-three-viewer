$( document ).ready(function() {
    const BLOG_CHOICE = $("#blog-choice");
    const SEARCH = $("#search");
    const PAGE_CHOICE = $("#page-choice");
    const POSTS_DIV = $("#posts");
    const PAGE_SIZE = 100;
    let POSTS_DATA = [];
    let POSTS_DATA_PROCESSED = [];

    $("#form").trigger("reset");

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

    function update_page_choice() {
        PAGE_CHOICE.empty()
        if (POSTS_DATA.length < 1) {
            const placeholder = new Option("1", "1");
            placeholder.setAttribute('disabled', true);
            PAGE_CHOICE.append(placeholder)
            PAGE_CHOICE.attr('disabled' , true);
        } else {
            const pages = Math.ceil(POSTS_DATA.length / PAGE_SIZE)
            for (let i = 1; i <= pages; i++) {
                PAGE_CHOICE.append(new Option(i.toString(), i.toString()));
            }
            PAGE_CHOICE.attr('disabled' , false);
        }
    }

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

    BLOG_CHOICE.change(function() {
        const blog = $(this).val();
        console.log("Loading blog files: " + blog);
        const requests = [
            Image.load(blog),
            Text.load(blog),
            Answer.load(blog),
            Video.load(blog),
        ]
        $.when(...requests).then((...responses) => {
            POSTS_DATA = responses.flat()
            update_page_choice();
            render_posts();
            SEARCH.attr('disabled' , false);
        }).catch((e) => {
            alert(e);
        })
    });

    PAGE_CHOICE.change(function() {
        render_posts();
    });

    SEARCH.change(function(e) {
        clearTimeout(this.thread);
        this.thread = setTimeout(function(){

        }, 250);
    });

    function render_posts() {
        POSTS_DIV.empty();
        POSTS_DATA.sort(function (a, b) {
            return a.id - b.id;
        });
        const page_number = parseInt(PAGE_CHOICE[0].value) - 1;
        const start = page_number * PAGE_SIZE
        const stop = (page_number + 1) * PAGE_SIZE
        for (const post of POSTS_DATA.slice(start, stop)) {
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

    constructor(lines) {
        this.id = parseInt(Post.line_starting_with(lines, Post.POST_ID));
        this.date = Post.line_starting_with(lines, Post.DATE);
        this.tags = Post.line_starting_with(lines, Post.TAGS);
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
    static split_posts(all_posts) {
        if (all_posts == null) {
            return []
        }
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
    // Inclusive to include the contents of the left line
    // Set right as null to collect all remaining lines
    static contents_between(lines, left, right) {
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
        return `<p>${this.tags}</p>`
    }

}

class Image extends Post {
    photo_urls;
    caption;

    static PHOTO_URL = "Photo url: "
    static PHOTO_SET_URLS = "Photo set urls: "
    static PHOTO_CAPTION = "Photo caption: "

    constructor(lines, blog) {
        super(lines);
        const photo_set_urls = Post.line_starting_with(lines, Image.PHOTO_SET_URLS).split(" ").filter((u) => u.length > 0)
        if (photo_set_urls.length > 0) {
            this.photo_urls = photo_set_urls
        } else {
            this.photo_urls = [Post.line_starting_with(lines, Image.PHOTO_URL)]
        }
        this.photo_urls = this.photo_urls.map((u) => Post.fix_url(u, blog))
        this.caption = Post.line_starting_with(lines, Image.PHOTO_CAPTION)
    }

    static load(blog) {
        return Post.get_blog_file(blog, "images.txt").then((res) => {
            return convert_posts(res, (lines) => new Image(lines, blog), "images")
        })
    }

    render() {
        const header = super.render_header();
        const images = this.photo_urls.map((u) => `<img src="${u}" alt="">`).join("\n")
        const footer = super.render_footer();
        return [header, this.caption, images, footer].join("\n")
    }

}

class Video extends Post {
    url;
    caption;

    static VIDEO_CAPTION = "Video caption: "
    static VIDEO_PLAYER = "Video player: "

    constructor(lines, blog) {
        super(lines);
        const player = Post.contents_between(lines, Video.VIDEO_PLAYER);
        const html = $.parseHTML( player );
        const url = html[0].children[0].attributes["src"].value;
        this.caption = Post.line_starting_with(lines, Video.VIDEO_CAPTION);
        this.url = Video.fix_url(url, blog)
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
            return convert_posts(res, (lines) => new Video(lines, blog), "videos")
        })
    }

    render() {
        const header = super.render_header();
        const footer = super.render_footer();
        const video = `<video controls><source src="${this.url}"></video>`
        return [header, this.caption, video, footer].join("\n")
    }
}

class Text extends Post {
    title;
    body;

    static TITLE = "Title: "

    constructor(lines) {
        super(lines);
        this.title = Post.line_starting_with(lines, Text.TITLE)
        this.body = Post.contents_between(lines, Text.TITLE, Post.TAGS)
    }

    static load(blog) {
        return Post.get_blog_file(blog, "texts.txt").then((res) => {
            return convert_posts(res, (lines) => new Text(lines), "texts")
        })
    }

    render() {
        const header = super.render_header();
        const footer = super.render_footer();
        const title = `<h4>${this.title}</h4>`
        return [header, title, this.body, footer].join("\n")
    }

}

class Answer extends Post {
    body;

    constructor(lines) {
        super(lines);
        this.body = Post.contents_between(lines, Post.REBLOG_NAME, Post.TAGS)
    }

    static load(blog) {
        return Post.get_blog_file(blog, "answers.txt").then((res) => {
            return convert_posts(res, (lines) => new Answer(lines), "answers")
        })
    }

    render() {
        const header = super.render_header();
        const footer = super.render_footer();
        return [header, this.body, footer].join("\n")
    }
}

function convert_posts(text, mapper, context) {
    const split = Post.split_posts(text);
    let posts = []
    for (const post of split) {
        try {
            posts.push(mapper(post))
        } catch (e) {
            const id = post[0]
            console.warn(`Unable to convert post: ${id}, due to: ${e}, context: ${context}`)
        }
    }
    if (posts.length) {
        console.log({
            [context]: posts,
        })
    }
    return posts
}
