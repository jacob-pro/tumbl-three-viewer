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
    const SORT = $("#sort")

    let ALL_POSTS = [];
    let FILTERED_POSTS = [];

    FORM.trigger("reset");
    FORM.submit(function( event ) {
        event.preventDefault();
    });

    $.get( BASE_URL + "/blogs" ).then(
        function(list) {
            if (list.length < 1) {
                throw new Error("No blogs found")
            }
            BLOG_CHOICE.empty();
            const placeholder = new Option("Choose a blog", "");
            placeholder.setAttribute('disabled', true);
            placeholder.setAttribute('selected', true);
            BLOG_CHOICE.append(placeholder);
            list.forEach((d) => BLOG_CHOICE.append(new Option(d,d)));
            BLOG_CHOICE.attr('disabled' , false);
        }
    ).catch((e) => {
        alert(e.responseText);
    })

    BLOG_CHOICE.change(function() {
        const blog = $(this).val();
        $.get( BASE_URL + "/blogs/" + blog ).then((posts) => {
            ALL_POSTS = posts.map(Post.deserialize);
            TOTAL.text(`Total: ${ALL_POSTS.length}`);
            apply_filters();
            update_page_choice();
            render_posts();
        }).catch((e) => {
            alert(e.responseText);
        })
    });

    PAGE_CHOICE.change(function() {
        apply_filters();
        render_posts();
    });

    TYPE.change(function() { refresh() });

    SORT.change(function() { refresh() });

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
        const sort = SORT[0].value;
        FILTERED_POSTS = ALL_POSTS.filter((p) => {
            return type === "All" || p.types().includes(type)
        })
        FILTERED_POSTS = FILTERED_POSTS.filter((p) => {
            return search.length === 0 || p.matches_search(search)
        })
        FILTERED_POSTS.sort(function (a, b) {
            if (sort === "Oldest") {
                return a.id - b.id;
            } else {
                return b.id - a.id;
            }
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
    post_url;

    constructor(json) {
        this.id = json["id"]
        this.date = json["date"]
        this.tags = json["tags"].join(", ")
        this.post_url = json["post_url"];
    }

    static deserialize(json) {
        const type = json["type"]
        if (type === PostType.Image) {
            return new Image(json);
        } else if (type === PostType.Video) {
            return new Video(json);
        } else if (type === PostType.Text) {
            return new Text(json);
        } else if (type === PostType.Answer) {
            return new Answer(json);
        } else {
            throw new Error("Unknown type " + type);
        }
    }

    render_header() {
        return `<p><a href="${this.post_url}">${this.date}</a></p>`
    }

    render_footer() {
        if (this.tags.length > 0) {
            return `<p>Tags: ${this.tags}</p>`
        }
        return ""
    }

    matches_search(search) {
        return this.tags.includes(search)
    }
}

class Image extends Post {
    photo_urls;
    caption;

    constructor(json) {
        super(json);
        this.photo_urls = json["photo_urls"]
        this.caption = json["caption"]
    }

    render() {
        const header = super.render_header();
        const images = this.photo_urls.map(render_image).join("\n")
        const footer = super.render_footer();
        return [header, this.caption, images, footer].join("\n")
    }

    matches_search(search) {
        return super.matches_search(search) || this.caption?.includes(search)
    }

    types() {
        return [PostType.Image]
    }
}

class Video extends Post {
    url;
    caption;

    constructor(json) {
        super(json)
        this.url = json["url"]
        this.caption = json["caption"]
    }

    render() {
        const header = super.render_header();
        const footer = super.render_footer();
        const caption = `<div>${this.caption}</div>`
        const video = render_video(this.url);
        return [header, caption, video, footer].join("\n")
    }

    matches_search(search) {
        return super.matches_search(search) || this.caption?.includes(search)
    }

    types() {
        return [PostType.Video]
    }
}

class Text extends Post {
    title;
    body;
    media_urls;

    constructor(json) {
        super(json);
        this.title = json["title"]
        this.body = json["body"]
        this.media_urls = json["media_urls"]
    }

    render() {
        const header = super.render_header();
        const footer = super.render_footer();
        const title = this.title ? `<h4>${this.title}</h4>` : "";
        const media = this.media_urls.map(render_text_media).join("\n")
        return [header, title, this.body, media, footer].join("\n")
    }

    matches_search(search) {
        return super.matches_search(search) || this.title?.includes(search) || this.body.includes(search)
    }

    types() {
        const out = [PostType.Text];
        if (this.media_urls.some(url => get_url_type(url) === TextMediaType.Video)) {
            out.push(PostType.Video)
        } else if (this.media_urls.some(url => get_url_type(url) === TextMediaType.Image)) {
            out.push(PostType.Image)
        }
        return out;
    }
}

class Answer extends Post {
    body;

    constructor(json) {
        super(json);
        this.body = json["body"]
    }

    render() {
        const header = super.render_header();
        const footer = super.render_footer();
        return [header, this.body, footer].join("\n")
    }

    matches_search(search) {
        return super.matches_search(search) || this.body.includes(search)
    }

    types() {
        return [PostType.Answer]
    }
}


function render_image(url) {
    return `<img src="${url}" alt="[image]">`;
}

function render_video(url) {
    return `<video controls><source src="${url}"></video>`;
}

function render_text_media(url) {
    const type = get_url_type(url);
    if (type === TextMediaType.Video) {
        return render_video(url);
    } else if (type === TextMediaType.Image) {
        return render_image(url);
    }
}

function get_url_type(url) {
    if (url.endsWith(".mp4")) {
        return TextMediaType.Video
    } else {
        return TextMediaType.Image
    }
}

const TextMediaType = {
    Video: 'Video',
    Image: 'Image',
};

const PostType = {
    Video: 'Video',
    Image: 'Image',
    Text: 'Text',
    Answer: 'Answer',
};
