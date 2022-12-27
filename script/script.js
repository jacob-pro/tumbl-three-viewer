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
        },
        function (e) {
            throw new Error("Get /blogs failed")
        }
    ).catch((e) => {
        alert(e);
    })

    BLOG_CHOICE.change(function() {
        const blog = $(this).val();
        $.get( BASE_URL + "/blogs/" + blog ).then((posts) => {
            ALL_POSTS = posts.map(Post.deserialize);
            console.log(ALL_POSTS);
            TOTAL.text(`Total: ${ALL_POSTS.length}`);
            apply_filters();
            update_page_choice();
            render_posts();
        }).catch((e) => {
            alert(e);
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
            return type === "All" || type === p.constructor.name
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

    constructor(id, date, tags) {
        this.id = id
        this.date = date
        this.tags = tags.join(", ")
    }

    static deserialize(post) {
        const type = post["type"]
        if (type === "Image") {
            return Image.deserialize(post);
        } else if (type === "Video") {
            return Video.deserialize(post);
        } else if (type === "Text") {
            return Text.deserialize(post);
        } else if (type === "Answer") {
            return Answer.deserialize(post);
        } else {
            throw new Error("Unknown type " + type);
        }
    }

    render_header() {
        return `<p>${this.date}</p>`
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

    constructor(id, date, tags, photo_urls, caption) {
        super(id, date, tags);
        this.photo_urls = photo_urls
        this.caption = caption
    }

    static deserialize(post) {
        return new Image(post["id"], post["date"], post["tags"], post["photo_urls"], post["caption"])
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

    constructor(id, date, tags, url, caption) {
        super(id, date, tags)
        this.url = url
        this.caption = caption
    }

    static deserialize(post) {
        return new Video(post["id"], post["date"], post["tags"], post["url"], post["caption"])
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

    constructor(id, date, tags, title, body) {
        super(id, date, tags);
        this.title = title
        this.body = body
    }

    static deserialize(post) {
        return new Text(post["id"], post["date"], post["tags"], post["title"], post["body"])
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

    static deserialize(post) {
        return new Answer(post["id"], post["date"], post["tags"], post["body"])
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
